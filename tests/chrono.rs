use chrono::{TimeZone, Utc};
use std::str::FromStr;
use std::time::SystemTime;

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
async fn systemtime_chrono_interop() {
    #[derive(Clone, Eq, Hash, PartialEq)]
    #[pco_store::store(timestamp = collected_at, group_by = [database_id])]
    pub struct QueryStat {
        pub database_id: i64,
        pub collected_at: SystemTime,
    }
    let database_id = 1;
    let start = Utc.with_ymd_and_hms(2026, 01, 01, 1, 00, 00).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 01, 01, 5, 00, 00).unwrap();
    let db = &DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id);
        CREATE INDEX ON query_stats USING btree (end_at, start_at);
    ";
    db.batch_execute(sql).await.unwrap();

    let time_1_23_45 = Utc.with_ymd_and_hms(2026, 01, 01, 1, 23, 45).unwrap();
    let time_2_34_56 = Utc.with_ymd_and_hms(2026, 01, 01, 2, 34, 56).unwrap();
    let time_3_45_00 = Utc.with_ymd_and_hms(2026, 01, 01, 3, 45, 00).unwrap();

    // Write, using SystemTime.
    let collected_at = time_1_23_45.into();
    let stats = vec![QueryStat { database_id, collected_at }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let collected_at = time_2_34_56.into();
    let stats = vec![QueryStat { database_id, collected_at }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read
    {
        let mut actual: Vec<QueryStat> = vec![];
        for group in CompressedQueryStats::load(db, &[database_id], start.into(), end.into()).await.unwrap() {
            for stat in group.decompress().unwrap() {
                actual.push(stat.clone());
            }
        }
        assert_eq!(actual.len(), 2);

        assert_eq!(actual[0].collected_at, time_1_23_45.into());
        assert_eq!(actual[1].collected_at, time_2_34_56.into());
    }

    {
        #[allow(dead_code)]
        #[derive(Clone, Eq, Hash, PartialEq)]
        #[pco_store::store(timestamp = collected_at, group_by = [database_id])]
        pub struct QueryStat {
            database_id: i64,
            collected_at: chrono::DateTime<Utc>,
        }

        // Read, using chrono::DateTime.
        let mut actual: Vec<QueryStat> = vec![];
        for group in CompressedQueryStats::load(db, &[database_id], start, end).await.unwrap() {
            for stat in group.decompress().unwrap() {
                actual.push(stat.clone());
            }
        }

        assert_eq!(actual.len(), 2);

        assert_eq!(actual[0].collected_at, time_1_23_45);
        assert_eq!(actual[1].collected_at, time_2_34_56);

        // Write, using chrono::DateTime.
        let collected_at = time_3_45_00;
        let stats = vec![QueryStat { database_id, collected_at }];
        CompressedQueryStats::store(db, stats).await.unwrap();
    }

    // Read again, using SystemTime.
    let mut actual: Vec<QueryStat> = vec![];
    for group in CompressedQueryStats::load(db, &[database_id], start.into(), end.into()).await.unwrap() {
        for stat in group.decompress().unwrap() {
            actual.push(stat.clone());
        }
    }

    assert_eq!(actual.len(), 3);

    assert_eq!(actual[0].collected_at, time_1_23_45.into());
    assert_eq!(actual[1].collected_at, time_2_34_56.into());
    assert_eq!(actual[2].collected_at, time_3_45_00.into());
}
