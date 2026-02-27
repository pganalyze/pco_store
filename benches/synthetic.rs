use ahash::AHashMap;
use peak_alloc::PeakAlloc;
use anyhow::Result;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};

mod synthetic {
    use super::*;
    pub mod pco_store;
}
use synthetic::*;


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
    println!("== pco_store");
    println!("=== store");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = pco_store::store().await?;
    println!("compressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());

    println!();
    println!("=== load (Vec)");

    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = pco_store::load().await?;
    println!("decompressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());

    println!();
    println!("=== load (reduce)");

    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let pco_store_duration = pco_store::load_reduce().await?;
    println!("decompressed after {:.1?} ({:.1?} in pco_store)", start.elapsed(), pco_store_duration);
    println!("peak memory usage: {:.0?}MB", PEAK_ALLOC.peak_usage_as_mb());


    Ok(())
}
