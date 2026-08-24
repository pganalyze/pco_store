use ahash::AHashMap;
use chrono::{DateTime, DurationRound, Utc};
use std::collections::hash_map::Entry;
use std::str::FromStr;
use std::time::SystemTime;

mod chrono_tests;
mod fields_tests;
mod filter_tests;
mod index_filter_tests;
mod serde_tests;
mod smol_str_tests;

pub static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
    if std::path::Path::new(".env").exists() {
        dotenvy::dotenv().unwrap();
    }
    let url = std::env::var("DATABASE_URL").unwrap_or("postgresql://localhost:5432/postgres".to_string());
    let pg_config = tokio_postgres::Config::from_str(&url).unwrap();
    let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
    let mgr = deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    deadpool_postgres::Pool::builder(mgr).build().unwrap().into()
});

#[tokio::test]
#[serial_test::serial]
async fn timestamp() {
    #[derive(Clone, Debug, PartialEq)]
    #[pco_store::store(timestamp = collected_at, index = [database_id])]
    pub struct QueryStat {
        pub database_id: i64,
        pub collected_at: DateTime<Utc>,
        pub fingerprint: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let start = DateTime::<Utc>::from(SystemTime::now() - std::time::Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap();
    let end = start + chrono::Duration::seconds(3600);

    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at);
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let collected_at = end - chrono::Duration::seconds(120);
    let stats = vec![QueryStat { database_id, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let collected_at = end - chrono::Duration::seconds(60);
    let stats = vec![QueryStat { database_id, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Load before delete so we can compare against what delete returns
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    // Delete and re-store to improve compression (store_grouped removed; pco_pack handles chunking)
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let deleted_chunks = CompressedQueryStats::delete(db, Filter::new([database_id], start..=end), &[]).await.unwrap();
    let mut deleted = Vec::new();
    for chunk in deleted_chunks {
        deleted.extend(chunk.decompress().unwrap());
    }
    assert_eq!(deleted.len(), 2);
    // Sort both to compare since chunk ordering may differ
    loaded.sort_by_key(|s| s.collected_at);
    deleted.sort_by_key(|s| s.collected_at);
    assert_eq!(deleted, loaded);
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    // Re-store all at once (pco_pack handles chunking internally)
    CompressedQueryStats::store(db, loaded.clone()).await.unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded[0].collected_at, end - chrono::Duration::seconds(120));
    assert_eq!(loaded[1].collected_at, end - chrono::Duration::seconds(60));

    // The `load` WHERE query and in-memory timestamp filter work as expected
    let mut stat = QueryStat { database_id, collected_at: start, fingerprint: 1, calls: 1, total_time: 1.0 };
    for e in 0..3 {
        println!("{e}");
        let mut stats = Vec::new();
        for _ in 1..=10 {
            stat.collected_at += chrono::Duration::seconds(60);
            stats.push(stat.clone());
        }
        CompressedQueryStats::store(db, stats).await.unwrap();
    }
    let start = start + chrono::Duration::seconds(3 * 60); // minute 3, skipping the first 2 minutes in the group
    let end = start + chrono::Duration::seconds(23 * 60); // minute 26, skipping the last 4 minutes in the group
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded.len(), 24);
    let (mut calls, mut min, mut max) = (0i64, DateTime::<Utc>::MAX_UTC, DateTime::<Utc>::MIN_UTC);
    for stat in loaded {
        calls += stat.calls;
        min = min.min(stat.collected_at);
        max = max.max(stat.collected_at);
    }
    assert_eq!((24i64, start, end), (calls, min, max));

    // Existing data can still be loaded when an empty `bytea` column is added to the table
    db.batch_execute("ALTER TABLE query_stats ADD COLUMN new_col bytea STORAGE EXTERNAL DEFAULT '' NOT NULL").await.unwrap();
    DB_POOL.manager().statement_caches.clear();
    {
        #[allow(dead_code)]
        #[derive(Clone)]
        #[pco_store::store(timestamp = collected_at, index = [database_id])]
        pub struct QueryStat {
            database_id: i64,
            collected_at: DateTime<Utc>,
            fingerprint: i64,
            calls: i64,
            total_time: f64,
            new_col: i32,
        }

        let end = end + chrono::Duration::seconds(5 * 60); // minute 31
        let stat = QueryStat { database_id, collected_at: end, fingerprint: 1, calls: 1, total_time: 1.0, new_col: 1 };
        CompressedQueryStats::store(db, vec![stat]).await.unwrap();
        let mut loaded = Vec::new();
        for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
            loaded.extend(chunk.decompress().unwrap());
        }
        assert_eq!(loaded.len(), 29);
        let (mut calls, mut new_col, mut min, mut max) = (0i64, 0i32, DateTime::<Utc>::MAX_UTC, DateTime::<Utc>::MIN_UTC);
        for stat in loaded {
            calls += stat.calls;
            new_col += stat.new_col;
            min = min.min(stat.collected_at);
            max = max.max(stat.collected_at);
        }
        assert_eq!((29i64, 1i32, start, end), (calls, new_col, min, max));
    }
}

// This test shows an intended use case of this crate: using a table partitioned by `granularity` to
// store both the original data received, and higher level aggregates needed to speed up read queries.
#[tokio::test]
#[serial_test::serial]
async fn aggregate() {
    #[pco_store::store(timestamp = collected_at, index = [database_id, granularity])]
    #[derive(Clone)]
    pub struct QueryStat {
        pub database_id: i64,
        pub granularity: i32,
        pub collected_at: DateTime<Utc>,
        pub fingerprint: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let start = DateTime::<Utc>::from(SystemTime::now() - std::time::Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap();
    let end = start + chrono::Duration::seconds(3600);

    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            granularity int NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        ) PARTITION BY LIST (granularity);
        CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
        CREATE TABLE query_stats_1min PARTITION OF query_stats FOR VALUES IN (60);
        CREATE TABLE query_stats_1hour PARTITION OF query_stats FOR VALUES IN (3600);
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let granularity = 60;
    let collected_at = start + chrono::Duration::seconds(10);
    let stats = vec![QueryStat { database_id, granularity, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let collected_at = start + chrono::Duration::seconds(20);
    let stats = vec![QueryStat { database_id, granularity, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], [granularity], start..=end), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Aggregate into hourly bucket
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let mut stats: AHashMap<_, QueryStat> = AHashMap::new();
    let start = DateTime::<Utc>::from(SystemTime::now() - std::time::Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap();
    let end = start + chrono::Duration::seconds(3600);
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], [granularity], start..=end), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            match stats.entry((stat.database_id, stat.fingerprint)) {
                Entry::Occupied(mut entry) => {
                    let e = entry.get_mut();
                    e.calls += stat.calls;
                    e.total_time += stat.total_time;
                }
                Entry::Vacant(entry) => {
                    let mut e = stat.clone();
                    e.granularity = 3600;
                    e.collected_at = start;
                    entry.insert(e);
                }
            }
        }
    }
    let stats: Vec<QueryStat> = stats.into_values().collect();
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, stats).await.unwrap();
    assert_eq!(3, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));

    // Load hourly aggregates using typed filter field for granularity
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id], [3600], start..=end), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].collected_at, start);
    assert_eq!(loaded[0].calls, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_timestamp() {
    #[pco_store::store(index = [database_id])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct QueryStat {
        pub database_id: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id);
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let stats = vec![QueryStat { database_id, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let stats = vec![QueryStat { database_id, calls: 2, total_time: 2.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);

    // Load before delete so we can compare against what delete returns
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    // Delete and re-store
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let deleted_chunks = CompressedQueryStats::delete(db, Filter::new([database_id]), &[]).await.unwrap();
    let mut deleted = Vec::new();
    for chunk in deleted_chunks {
        deleted.extend(chunk.decompress().unwrap());
    }
    assert_eq!(deleted.len(), 2);
    // Sort both to compare since chunk ordering may differ
    loaded.sort_by_key(|s| s.calls);
    deleted.sort_by_key(|s| s.calls);
    assert_eq!(deleted, loaded);
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, loaded.clone()).await.unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded[0].calls, 1);
    assert_eq!(loaded[1].calls, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_index() {
    #[pco_store::store]
    #[derive(Clone, Debug, PartialEq)]
    pub struct QueryStat {
        pub database_id: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let stats = vec![QueryStat { database_id, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let stats = vec![QueryStat { database_id, calls: 2, total_time: 2.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for chunk in CompressedQueryStats::load(db, Filter::default(), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);

    // Load before delete so we can compare against what delete returns
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::default(), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    // Delete and re-store
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let deleted_chunks = CompressedQueryStats::delete(db, Filter::default(), &[]).await.unwrap();
    let mut deleted = Vec::new();
    for chunk in deleted_chunks {
        deleted.extend(chunk.decompress().unwrap());
    }
    assert_eq!(deleted.len(), 2);
    // Sort both to compare since chunk ordering may differ
    loaded.sort_by_key(|s| s.calls);
    deleted.sort_by_key(|s| s.calls);
    assert_eq!(deleted, loaded);
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, loaded.clone()).await.unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    for chunk in CompressedQueryStats::load(db, Filter::default(), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded[0].calls, 1);
    assert_eq!(loaded[1].calls, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn table_name() {
    #[pco_store::store(table_name = other)]
    pub struct QueryStat {
        pub database_id: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS other;
        CREATE TABLE other (
            database_id bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let stats = vec![QueryStat { database_id, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let stats = vec![QueryStat { database_id, calls: 2, total_time: 2.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for chunk in CompressedQueryStats::load(db, Filter::default(), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);
}

#[tokio::test]
#[serial_test::serial]
async fn float_round() {
    #[pco_store::store(index = [database_id], float_round = 2)]
    pub struct QueryStat {
        pub database_id: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let database_id = 1;
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let stats = vec![QueryStat { database_id, calls: 1, total_time: 1.2345 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let stats = vec![QueryStat { database_id, calls: 2, total_time: 1.2345 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut total_time = 0.0;
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
        for stat in chunk.decompress().unwrap() {
            total_time += stat.total_time;
        }
    }
    assert_eq!(total_time, 2.46);

    DB_POOL.manager().statement_caches.clear();
    {
        #[pco_store::store(index = [database_id], float_round = 3)]
        pub struct QueryStat {
            pub database_id: i64,
            pub calls: i64,
            pub total_time: f64,
        }

        let database_id = 1;
        let db = &DB_POOL.get().await.unwrap();
        let sql = "
            DROP TABLE IF EXISTS query_stats;
            CREATE TABLE query_stats (
                database_id bigint NOT NULL,
                calls bytea STORAGE EXTERNAL NOT NULL,
                total_time bytea STORAGE EXTERNAL NOT NULL
            );
        ";
        db.batch_execute(sql).await.unwrap();

        // Write
        let stats = vec![QueryStat { database_id, calls: 1, total_time: 1.2345 }];
        CompressedQueryStats::store(db, stats).await.unwrap();
        let stats = vec![QueryStat { database_id, calls: 2, total_time: 1.2345 }];
        CompressedQueryStats::store(db, stats).await.unwrap();

        // Read
        let mut total_time = 0.0;
        for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
            for stat in chunk.decompress().unwrap() {
                total_time += stat.total_time;
            }
        }
        // If the floats were simply truncated this would be 2.468, but rounding gets it to 2.47
        assert_eq!(total_time, 2.47);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn boolean() {
    #[pco_store::store(index = [database_id])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct QueryStat {
        pub database_id: i64,
        pub calls: i64,
        pub toplevel: bool,
    }

    let database_id = 1;
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            toplevel bytea STORAGE EXTERNAL NOT NULL
        );
    ";
    db.batch_execute(sql).await.unwrap();

    // Write
    let stats = vec![QueryStat { database_id, calls: 1, toplevel: true }, QueryStat { database_id, calls: 2, toplevel: false }];
    CompressedQueryStats::store(db, stats.clone()).await.unwrap();

    // Read
    let mut loaded = Vec::new();
    for chunk in CompressedQueryStats::load(db, Filter::new([database_id]), &[]).await.unwrap() {
        loaded.extend(chunk.decompress().unwrap());
    }
    assert_eq!(loaded, stats);
}
