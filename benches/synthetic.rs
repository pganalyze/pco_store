use ahash::AHashMap;
use anyhow::Result;
use chrono::{DateTime, Utc};
use markdown_tables::MarkdownTableRow;
use pco_store::pg::Postgres;
use pco_store::{PcoStore, TryStreamExt, pco_store};
use peak_alloc::PeakAlloc;
use std::str::FromStr;
use std::time::{Duration, Instant};

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
    let mut rows = Vec::new();
    println!("== pco_store");

    println!();
    println!("=== store");
    PEAK_ALLOC.reset_peak_usage();
    let duration = store().await?;
    rows.push(BenchRow::new("store", duration));

    println!();
    println!("=== load (100% of rows)");
    PEAK_ALLOC.reset_peak_usage();
    let duration = load().await?;
    rows.push(BenchRow::new("load (100% of rows)", duration));

    println!();
    println!("=== reduce (100% of rows)");
    PEAK_ALLOC.reset_peak_usage();
    let duration = reduce().await?;
    rows.push(BenchRow::new("reduce (100% of rows)", duration));

    println!();
    println!("=== filter ({} databases + fingerprint range, {:.3}% of rows)", FILTER_DATABASE_COUNT, filter_percent());
    PEAK_ALLOC.reset_peak_usage();
    let duration = filter().await?;
    rows.push(BenchRow::new(format!("filter ({} databases + fingerprint range, {:.3}%)", FILTER_DATABASE_COUNT, filter_percent()), duration));

    println!();
    println!("=== fold ({:.3}% of rows)", filter_percent());
    fold(&mut rows).await?;

    println!();
    print!("{}", markdown_tables::as_table(&rows));

    Ok(())
}

/// One rendered scenario: time inside pco_store and peak allocator usage
/// when the scenario finished. The whole run prints as one markdown table
/// at the end, which is what benches/README.md records.
struct BenchRow {
    operation: String,
    time: String,
    peak: String,
}

impl BenchRow {
    fn new(operation: impl Into<String>, duration: Duration) -> Self {
        let time = if duration.as_secs_f64() >= 1.0 {
            format!("{:.1} s", duration.as_secs_f64())
        } else {
            format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
        };
        Self { operation: operation.into(), time, peak: format!("{:.0} MB", PEAK_ALLOC.peak_usage_as_mb()) }
    }
}

impl MarkdownTableRow for BenchRow {
    fn column_names() -> Vec<&'static str> {
        vec!["Operation", "Time", "Peak memory"]
    }

    fn column_values(&self) -> Vec<String> {
        vec![self.operation.clone(), self.time.clone(), self.peak.clone()]
    }
}

#[derive(Clone, PcoStore)]
#[pco_store(name = synthetic_pco_stores, timestamp = collected_at, index = [database_id], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: DateTime<Utc>,
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

type Filter = pco_store::Filter<QueryStat>;

/// Synthetic data scale: 1M rows per database is four 262,144-row chunks per
/// key, so every read scenario reads many chunks per key, and multi-value
/// index filters take the `= ANY($n)` path a real workload takes.
const DATABASE_COUNT: usize = 100;
const ROWS_PER_DATABASE: usize = 1_000_000;
/// The `filter` scenario (and the fold comparison, which shares its filter)
/// reads three databases' chunks and keeps 16 fingerprint values in 500,
/// which lands just under 0.1% of rows matched.
const FILTER_DATABASE_COUNT: usize = 3;
const FINGERPRINT_MODULUS: usize = 500;
const FINGERPRINT_MATCH_COUNT: usize = 16;

/// The ids and fingerprints the filter and fold scenarios share, so the fold
/// comparison runs against the same selective query the filter scenario
/// prices.
fn match_filter() -> Filter {
    let mut filter = Filter::new(vec![50i64, 51, 52], DateTime::<Utc>::UNIX_EPOCH..=Utc::now());
    filter.fingerprint = Some((200..=215).into());
    filter
}

fn filter_percent() -> f64 {
    FILTER_DATABASE_COUNT as f64 * ROWS_PER_DATABASE as f64 * FINGERPRINT_MATCH_COUNT as f64
        / FINGERPRINT_MODULUS as f64
        / (DATABASE_COUNT * ROWS_PER_DATABASE) as f64
        * 100.0
}

fn pg() -> Postgres {
    Postgres::new(DB_POOL.clone())
}

static DEFAULT_COLLECTED_AT: std::sync::LazyLock<DateTime<Utc>> = std::sync::LazyLock::new(Utc::now);

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
    for db_id in 0..DATABASE_COUNT as i64 {
        for i in 0..ROWS_PER_DATABASE as i64 {
            stats.push(QueryStat {
                database_id: db_id,
                collected_at: Utc::now(),
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
    let pg = pg();
    QueryStat::store(&pg, stats).await?;

    Ok(start.elapsed())
}

pub async fn load() -> Result<Duration> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM synthetic_pco_stores", &[]).await?.get(0);
    let mut stats = Vec::new();
    let filter = Filter::new(database_ids, DateTime::<Utc>::UNIX_EPOCH..=Utc::now());
    let start = Instant::now();
    for chunk in QueryStat::load(db, &filter, &[]).try_collect::<Vec<_>>().await? {
        stats.extend(chunk.decompress()?);
    }
    return Ok(start.elapsed());
}

pub async fn reduce() -> Result<Duration> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM synthetic_pco_stores", &[]).await?.get(0);
    let mut stats: AHashMap<(i64, i64, i64), QueryStat> = AHashMap::new();
    let filter = Filter::new(database_ids, DateTime::<Utc>::UNIX_EPOCH..=Utc::now());
    let start = Instant::now();
    for chunk in QueryStat::load(db, &filter, &[]).try_collect::<Vec<_>>().await? {
        for stat in chunk.decompress()? {
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
    }

    return Ok(start.elapsed());
}

// Three databases and a 16-value fingerprint range: the multi-value
// inclusion emits `database_id = ANY($n)`, so the read takes the
// array-parameter path over several chunks per key, and the fingerprint
// range drops all but a tenth of one percent of rows inside each chunk.
pub async fn filter() -> Result<Duration> {
    let pg = pg();
    let filter = match_filter();
    let start = Instant::now();
    let mut count = 0;
    for chunk in QueryStat::load(&pg, &filter, &[]).try_collect::<Vec<_>>().await? {
        count += chunk.decompress()?.len();
    }
    assert_eq!(count, FILTER_DATABASE_COUNT * ROWS_PER_DATABASE * FINGERPRINT_MATCH_COUNT / FINGERPRINT_MODULUS);
    return Ok(start.elapsed());
}

/// The same `fold!` projection through serial `for_each` and
/// `for_each_parallel` over the filter scenario's selective query, so one
/// process holds both sides of the comparison against the same pco_pack:
/// serial is the before column, each worker count an after. The projection
/// spans six payload columns of the rows that survive the filter.
async fn fold(rows: &mut Vec<BenchRow>) -> Result<()> {
    let db = &DB_POOL.get().await.unwrap();
    let filter = match_filter();

    PEAK_ALLOC.reset_peak_usage();
    let (serial_ms, serial_sums) = fold_run(db, &filter, None).await?;
    println!("serial for_each: {:.1?}", serial_ms);
    rows.push(BenchRow::new(format!("fold ({:.3}% of rows, serial)", filter_percent()), serial_ms));
    for workers in [2, 4] {
        PEAK_ALLOC.reset_peak_usage();
        let (ms, sums) = fold_run(db, &filter, Some(std::num::NonZeroUsize::new(workers).unwrap())).await?;
        assert_eq!(sums, serial_sums, "parallel fold must equal the serial fold at {workers} workers");
        println!("for_each_parallel({workers}): {:.1?} ({}x)", ms, serial_ms.as_secs_f64() / ms.as_secs_f64());
        rows.push(BenchRow::new(format!("fold (`for_each_parallel`, {workers})"), ms));
    }
    Ok(())
}

/// Folds every column the pipeline should decode and returns wall time plus
/// the six sums, so callers can assert serial and parallel agreement. Order
/// is load order in both paths, so even the float sums agree exactly.
async fn fold_run(
    db: &deadpool_postgres::Object, filter: &Filter, workers: Option<std::num::NonZeroUsize>,
) -> Result<(Duration, (i64, i64, f64, f64, i64, i64))> {
    let mut calls_sum = 0i64;
    let mut rows_sum = 0i64;
    let mut total_time_sum = 0f64;
    let mut io_time_sum = 0f64;
    let mut hits_sum = 0i64;
    let mut reads_sum = 0i64;
    let start = Instant::now();
    let consumer =
        pco_store::fold!(QueryStat, |calls: i64, rows: i64, total_time: f64, io_time: f64, shared_blks_hit: i64, shared_blks_read: i64| {
            calls_sum += calls;
            rows_sum += rows;
            total_time_sum += total_time;
            io_time_sum += io_time;
            hits_sum += shared_blks_hit;
            reads_sum += shared_blks_read;
        });
    match workers {
        None => QueryStat::for_each(db, filter, &[], consumer).await?,
        Some(workers) => QueryStat::for_each_parallel(db, filter, &[], consumer, workers).await?,
    }
    let ms = start.elapsed();
    Ok((ms, (calls_sum, rows_sum, total_time_sum, io_time_sum, hits_sum, reads_sum)))
}
