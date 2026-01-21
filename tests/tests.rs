use ahash::AHashMap;
use chrono::{DateTime, DurationRound, Utc};
use std::collections::hash_map::Entry;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[test]
fn macrotest() {
    macrotest::expand("tests/expand/query_stats.rs");
    macrotest::expand("tests/expand/boolean.rs");
    macrotest::expand("tests/expand/no_group_by.rs");
    macrotest::expand("tests/expand/float_round.rs");
}

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
    #[derive(Clone)]
    #[pco_store::store(timestamp = collected_at, group_by = [database_id])]
    pub struct QueryStat {
        pub database_id: i64,
        pub collected_at: SystemTime,
        pub fingerprint: i64,
        pub calls: i64,
        pub total_time: f64,
    }
    let database_id = 1;
    let start: SystemTime =
        DateTime::<Utc>::from(SystemTime::now() - Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap().into();
    let end = start + Duration::from_secs(3600);
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
    let collected_at = end - Duration::from_secs(120);
    let stats = vec![QueryStat { database_id, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let collected_at = end - Duration::from_secs(60);
    let stats = vec![QueryStat { database_id, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for group in CompressedQueryStats::load(db, &[database_id], start, end).await.unwrap() {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Delete and re-group to improve compression
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let mut stats = Vec::new();
    for group in CompressedQueryStats::delete(db, &[database_id], start, end).await.unwrap() {
        stats.extend(group.decompress().unwrap());
    }
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store_grouped(db, stats, |stat| {
        let collected_at: chrono::DateTime<chrono::Utc> = stat.collected_at.into();
        collected_at.duration_trunc(chrono::Duration::days(1)).ok()
    })
    .await
    .unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, &[database_id], start, end).await.unwrap().remove(0);
    assert_eq!(group.start_at, end - Duration::from_secs(120));
    assert_eq!(group.end_at, end - Duration::from_secs(60));
    let stats = group.decompress().unwrap();
    assert_eq!(stats[0].collected_at, end - Duration::from_secs(120));
    assert_eq!(stats[1].collected_at, end - Duration::from_secs(60));

    // The `load` WHERE query and `decompress` timestamp filter work as expected
    let mut stat = QueryStat { database_id, collected_at: start, fingerprint: 1, calls: 1, total_time: 1.0 };
    for e in 0..3 {
        println!("{e}");
        let mut stats = Vec::new();
        for _ in 1..=10 {
            stat.collected_at += Duration::from_secs(60);
            stats.push(stat.clone());
        }
        CompressedQueryStats::store(db, stats).await.unwrap();
    }
    let start = start + Duration::from_secs(3 * 60); // minute 3, skipping the first 2 minutes in the group
    let end = start + Duration::from_secs(23 * 60); // minute 26, skipping the last 4 minutes in the group
    let groups = CompressedQueryStats::load(db, &[database_id], start, end).await.unwrap();
    assert_eq!(3, groups.len());
    let (mut calls, mut min, mut max) = (0, SystemTime::now(), SystemTime::UNIX_EPOCH);
    for group in groups {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
            min = min.min(stat.collected_at);
            max = max.max(stat.collected_at);
        }
    }
    assert_eq!((24, start, end), (calls, min, max));

    // Existing data can still be loaded when an empty `bytea` column is added to the table
    db.batch_execute("ALTER TABLE query_stats ADD COLUMN new_col bytea STORAGE EXTERNAL DEFAULT '' NOT NULL").await.unwrap();
    DB_POOL.manager().statement_caches.clear();
    {
        #[allow(dead_code)]
        #[derive(Clone)]
        #[pco_store::store(timestamp = collected_at, group_by = [database_id])]
        pub struct QueryStat {
            database_id: i64,
            collected_at: SystemTime,
            fingerprint: i64,
            calls: i64,
            total_time: f64,
            new_col: i32,
        }
        let end = end + Duration::from_secs(5 * 60); // minute 31
        let stat = QueryStat { database_id, collected_at: end, fingerprint: 1, calls: 1, total_time: 1.0, new_col: 1 };
        CompressedQueryStats::store(db, vec![stat]).await.unwrap();
        let groups = CompressedQueryStats::load(db, &[database_id], start, end).await.unwrap();
        assert_eq!(4, groups.len());
        let (mut calls, mut new_col, mut min, mut max) = (0, 0, SystemTime::now(), SystemTime::UNIX_EPOCH);
        for group in groups {
            for stat in group.decompress().unwrap() {
                calls += stat.calls;
                new_col += stat.new_col;
                min = min.min(stat.collected_at);
                max = max.max(stat.collected_at);
            }
        }
        assert_eq!((29, 1, start, end), (calls, new_col, min, max));
    }
}

// This test shows an intended use case of this crate: using a table partitioned by `granularity` to
// store both the original data received, and higher level aggregates needed to speed up read queries.
#[tokio::test]
#[serial_test::serial]
async fn aggregate() {
    #[pco_store::store(timestamp = collected_at, group_by = [database_id, granularity])]
    pub struct QueryStat {
        pub database_id: i64,
        pub granularity: i32,
        pub collected_at: SystemTime,
        pub fingerprint: i64,
        pub calls: i64,
        pub total_time: f64,
    }
    let database_id = 1;
    let start: SystemTime =
        DateTime::<Utc>::from(SystemTime::now() - Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap().into();
    let end = start + Duration::from_secs(3600);
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
    let collected_at = start + Duration::from_secs(10);
    let stats = vec![QueryStat { database_id, granularity, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let collected_at = start + Duration::from_secs(20);
    let stats = vec![QueryStat { database_id, granularity, collected_at, fingerprint: 1, calls: 1, total_time: 1.0 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    let mut calls = 0;
    for group in CompressedQueryStats::load(db, &[database_id], &[granularity], start, end).await.unwrap() {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Aggregate into hourly bucket
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let mut stats: AHashMap<_, QueryStat> = AHashMap::new();
    let start: SystemTime = DateTime::<Utc>::from(end - Duration::from_secs(3600)).duration_trunc(chrono::Duration::hours(1)).unwrap().into();
    let end = start + Duration::from_secs(3600);
    for group in CompressedQueryStats::load(db, &[database_id], &[60], start, end).await.unwrap() {
        for stat in group.decompress().unwrap() {
            match stats.entry((stat.database_id, stat.fingerprint)) {
                Entry::Occupied(mut entry) => {
                    let e = entry.get_mut();
                    e.calls += stat.calls;
                    e.total_time += stat.total_time;
                }
                Entry::Vacant(entry) => {
                    let e = entry.insert(stat);
                    e.granularity = 3600;
                    e.collected_at = start;
                }
            }
        }
    }
    let stats: Vec<QueryStat> = stats.into_values().collect();
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, stats).await.unwrap();
    assert_eq!(3, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, &[database_id], &[3600], start, end).await.unwrap().remove(0);
    assert_eq!(group.start_at, start);
    assert_eq!(group.end_at, start);
    let stats = group.decompress().unwrap();
    assert_eq!(stats[0].collected_at, start);
    assert_eq!(stats[0].calls, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_timestamp() {
    #[pco_store::store(group_by = [database_id])]
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
    for group in CompressedQueryStats::load(db, &[database_id]).await.unwrap() {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);

    // Delete and re-group
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let mut stats = Vec::new();
    for group in CompressedQueryStats::delete(db, &[database_id]).await.unwrap() {
        for stat in group.decompress().unwrap() {
            stats.push(stat);
        }
    }
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, stats).await.unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, &[database_id]).await.unwrap().remove(0);
    let stats = group.decompress().unwrap();
    assert_eq!(stats[0].calls, 1);
    assert_eq!(stats[1].calls, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn no_group_by() {
    #[pco_store::store]
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
    for group in CompressedQueryStats::load(db).await.unwrap() {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);

    // Delete and re-group
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let mut stats = Vec::new();
    for group in CompressedQueryStats::delete(db).await.unwrap() {
        for stat in group.decompress().unwrap() {
            stats.push(stat);
        }
    }
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    CompressedQueryStats::store(db, stats).await.unwrap();
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await.unwrap().get::<_, i64>(0));
    let group = CompressedQueryStats::load(db).await.unwrap().remove(0);
    let stats = group.decompress().unwrap();
    assert_eq!(stats[0].calls, 1);
    assert_eq!(stats[1].calls, 2);
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
    for group in CompressedQueryStats::load(db).await.unwrap() {
        for stat in group.decompress().unwrap() {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 3);
}

#[tokio::test]
#[serial_test::serial]
async fn float_round() {
    #[pco_store::store(group_by = [database_id], float_round = 2)]
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
    for group in CompressedQueryStats::load(db, &[database_id]).await.unwrap() {
        for stat in group.decompress().unwrap() {
            total_time += stat.total_time;
        }
    }
    assert_eq!(total_time, 2.46);

    DB_POOL.manager().statement_caches.clear();
    {
        #[pco_store::store(group_by = [database_id], float_round = 3)]
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
        for group in CompressedQueryStats::load(db, &[database_id]).await.unwrap() {
            for stat in group.decompress().unwrap() {
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
    #[pco_store::store(group_by = [database_id])]
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
    let group = CompressedQueryStats::load(db, &[database_id]).await.unwrap().remove(0);
    assert_eq!(stats, group.decompress().unwrap());
}
