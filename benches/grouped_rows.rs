#![allow(dead_code)]

use ahash::AHashMap;
use anyhow::Result;
use brunch::{Bench, Benches};
use chrono::{DateTime, DurationRound, Utc};
use deadpool_postgres::Client;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};

static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
    dotenvy::dotenv().unwrap();
    let url = std::env::var("DATABASE_URL").unwrap_or("postgresql://localhost:5432/postgres".to_string());
    let pg_config = tokio_postgres::Config::from_str(&url).unwrap();
    let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
    let mgr = deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    deadpool_postgres::Pool::builder(mgr).build().unwrap().into()
});


fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut benches = Benches::default();

    runtime.block_on(async {
        let db = &mut DB_POOL.get().await.unwrap();
        let sql = "
            DROP TABLE IF EXISTS bucket_size_one_day;
            CREATE TABLE bucket_size_one_day (
                database_id bigint NOT NULL,
                start_at timestamptz NOT NULL,
                end_at timestamptz NOT NULL,
                collected_at bytea STORAGE EXTERNAL NOT NULL,
                collected_secs bytea STORAGE EXTERNAL NOT NULL,
                fingerprint bytea STORAGE EXTERNAL NOT NULL,
                postgres_role_id bytea STORAGE EXTERNAL NOT NULL,
                calls bytea STORAGE EXTERNAL NOT NULL,
                rows bytea STORAGE EXTERNAL NOT NULL,
                total_time bytea STORAGE EXTERNAL NOT NULL,
                io_time bytea STORAGE EXTERNAL NOT NULL,
                shared_blks_hit bytea STORAGE EXTERNAL NOT NULL,
                shared_blks_read bytea STORAGE EXTERNAL NOT NULL
            );
            CREATE INDEX ON bucket_size_one_day USING btree (database_id);
            CREATE INDEX ON bucket_size_one_day USING btree (end_at, start_at);
        ";
        db.batch_execute(sql).await.unwrap();
        let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM query_stats", &[]).await.unwrap().get(0);
        for database_id in database_ids {
            let sql = "
                SELECT start_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
                FROM query_stats WHERE database_id = $1
            ";
            let rows = db.query(sql, &[&database_id]).await.unwrap();

            benches.push(
                Bench::new(format!("HashMap {}", database_id))
                .run(|| {
                    let mut grouped_rows: HashMap<_, Vec<_>> = HashMap::new();
                    for row in &rows {
                        let time: DateTime<Utc> = row.get::<_, SystemTime>(0).into();
                        let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
                        grouped_rows.entry(date).or_default().push(row);
                    }
                })
            );

            benches.push(
                Bench::new(format!("AHashMap {}", database_id))
                .run(|| {
                    let mut grouped_rows: AHashMap<_, Vec<_>> = AHashMap::new();
                    for row in &rows {
                        let time: DateTime<Utc> = row.get::<_, SystemTime>(0).into();
                        let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
                        grouped_rows.entry(date).or_default().push(row);
                    }
                })
            );
        }
    });

    benches.finish();
}
