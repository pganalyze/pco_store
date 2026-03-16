use ahash::AHashMap;
use anyhow::Result;
use peak_alloc::PeakAlloc;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
    if std::path::Path::new(".env").exists() {
        dotenvy::dotenv().unwrap();
    }
    let url = std::env::var("DATABASE_URL").unwrap_or("postgresql://localhost:5432/postgres".to_string());
    let pg_config = tokio_postgres::Config::from_str(&url).unwrap();
    let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
    let mgr = deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    deadpool_postgres::Pool::builder(mgr).build().unwrap().into()
});

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("== synthetic");
    println!("=== store");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = store().await?;
    println!("compressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());

    println!();
    println!("=== load (Vec)");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = load().await?;
    println!("decompressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());

    println!();
    println!("=== load (reduce)");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = load_reduce().await?;
    println!("decompressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());

    Ok(())
}

#[::pco_store::store(timestamp = collected_at, group_by = [database_id], float_round = 2, table_name = synthetic_pco_stores)]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: SystemTime,
    pub collected_secs: i64,
    pub fingerprint: i64,
    pub postgres_role_id: i64,
    pub calls: i64,
    pub rows: i64,
    pub total_time: f64,
    pub io_time: f64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
}

static DEFAULT_COLLECTED_AT: std::sync::LazyLock<SystemTime> = std::sync::LazyLock::new(|| SystemTime::now());

impl Default for QueryStat {
    fn default() -> Self {
        Self {
            database_id: 0,
            collected_at: *DEFAULT_COLLECTED_AT,
            collected_secs: 0,
            fingerprint: 0,
            postgres_role_id: 0,
            calls: 0,
            rows: 0,
            total_time: 0.0,
            io_time: 0.0,
            shared_blks_hit: 0,
            shared_blks_read: 0,
        }
    }
}

pub async fn store() -> Result<Duration> {
    let db = &mut DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS synthetic_pco_stores;
        CREATE TABLE synthetic_pco_stores (
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
        CREATE INDEX ON synthetic_pco_stores USING btree (database_id, end_at, start_at);
    ";
    db.batch_execute(sql).await?;
    let mut stats = Vec::new();
    for db_id in 0..100 {
        for i in 0..100_000 {
            stats.push(QueryStat {
                database_id: db_id,
                collected_at: SystemTime::now(),
                collected_secs: 10,
                fingerprint: i % 500,
                postgres_role_id: i % 1_000,
                calls: 100 + i,
                rows: 10 + i,
                total_time: 1234.0,
                io_time: 12345.0,
                shared_blks_hit: 10 + i,
                shared_blks_read: 20 + i,
            });
        }
    }
    let start = Instant::now();
    CompressedQueryStats::store(db, stats).await?;
    Ok(start.elapsed())
}

pub async fn load() -> Result<Duration> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM synthetic_pco_stores", &[]).await?.get(0);
    let filter = Filter::new(&database_ids, SystemTime::UNIX_EPOCH..=SystemTime::now());
    let start = Instant::now();
    let _stats: Vec<_> = CompressedQueryStats::load(db, filter, ()).await?.collect();
    return Ok(start.elapsed());
}

pub async fn load_reduce() -> Result<Duration> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM synthetic_pco_stores", &[]).await?.get(0);
    let mut stats: AHashMap<(i64, i64, i64), QueryStat> = AHashMap::new();
    let filter = Filter::new(&database_ids, SystemTime::UNIX_EPOCH..=SystemTime::now());
    let start = Instant::now();
    for stat in CompressedQueryStats::load(db, filter, ()).await? {
        let key = (stat.database_id, stat.fingerprint, stat.postgres_role_id);
        let entry = stats.entry(key).or_default();
        entry.database_id = stat.database_id;
        entry.collected_at = stat.collected_at;
        entry.collected_secs += stat.collected_secs;
        entry.fingerprint = stat.fingerprint;
        entry.postgres_role_id = stat.postgres_role_id;
        entry.calls += stat.calls;
        entry.rows += stat.rows;
        entry.total_time += stat.total_time;
        entry.io_time += stat.io_time;
        entry.shared_blks_hit += stat.shared_blks_hit;
        entry.shared_blks_read += stat.shared_blks_read;
    }
    return Ok(start.elapsed());
}
