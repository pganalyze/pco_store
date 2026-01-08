#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, DurationRound, Utc};
use deadpool_postgres::Client;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};

mod comparison {
    use super::*;
    pub mod pco;
    pub mod pco_store;
    pub mod postgres;
}
use comparison::*;

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
    println!("====== comparison");

    println!("== pco");
    let start = Instant::now();
    pco::store().await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    pco::load().await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== pco_store");
    let start = Instant::now();
    pco_store::store().await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    pco_store::load().await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== postgres");
    let start = Instant::now();
    postgres::store().await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    postgres::load().await?;
    println!("decompressed after {:.1?}", start.elapsed());

    Ok(())
}
