#![allow(dead_code)]

use ahash::AHashMap;
use anyhow::Result;
use chrono::{DateTime, DurationRound, Utc};
use deadpool_postgres::Client;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};

mod bucket_size {
    use super::*;
    pub mod one_day;
    pub mod ten_minute;
}
use bucket_size::*;

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
    println!("====== bucket_size");

    println!("== 10 minute");
    let start = Instant::now();
    ten_minute::store().await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    ten_minute::load().await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== 1 day");
    let start = Instant::now();
    one_day::store().await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    one_day::load().await?;
    println!("decompressed after {:.1?}", start.elapsed());

    Ok(())
}
