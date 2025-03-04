#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, DurationRound, Utc};
use deadpool_postgres::Client;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};

mod float {
    use super::*;
    pub mod mult;
    pub mod round;
}
use float::*;

static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
    let url = std::env::var("DATABASE_URL").unwrap_or("postgresql://localhost:5432/postgres".to_string());
    let pg_config = tokio_postgres::Config::from_str(&url).unwrap();
    let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
    let mgr = deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    deadpool_postgres::Pool::builder(mgr).build().unwrap().into()
});

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("====== float");

    println!("== round(0)");
    let start = Instant::now();
    round::store(0).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    round::load(0).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== round(1)");
    let start = Instant::now();
    round::store(1).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    round::load(1).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== round(2)");
    let start = Instant::now();
    round::store(2).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    round::load(2).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== mult(1)");
    let start = Instant::now();
    mult::store(1).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    mult::load(1).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== mult(10)");
    let start = Instant::now();
    mult::store(10).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    mult::load(10).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    println!("== mult(100)");
    let start = Instant::now();
    mult::store(100).await?;
    println!("compressed after {:.1?}", start.elapsed());
    let start = Instant::now();
    mult::load(100).await?;
    println!("decompressed after {:.1?}", start.elapsed());

    Ok(())
}
