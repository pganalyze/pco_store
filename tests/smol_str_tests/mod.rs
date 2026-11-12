//! Integration tests for smol_str::SmolStr as an `index` field.
//!
//! Verifies that SmolStr index fields map to text in Postgres, generate correct SQL
//! filters (equality and inclusion via StringFilter), and round-trip values correctly.

use chrono::{DateTime, Utc};
use pco_store::pco_pack::PcoPack;
use pco_store::{Backend, PcoStore, pco_store};
use smol_str::SmolStr;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, PcoStore)]
#[pco_store(index = [database_id, region], timestamp = collected_at)]
pub struct SmolMetric {
    pub database_id: i64,
    pub region: SmolStr,
    pub collected_at: DateTime<Utc>,
    pub calls: i64,
}

type Filter = <SmolMetric as PcoPack>::Filter;

async fn setup_table() -> anyhow::Result<()> {
    let db = &super::DB_POOL.get().await?;
    db.batch_execute(
        "DROP TABLE IF EXISTS smol_metrics;
        CREATE TABLE smol_metrics (
            database_id bigint NOT NULL,
            region text NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL
        );",
    )
    .await?;
    Ok(())
}

async fn store_rows_with_regions(pg: &Arc<dyn Backend<SmolMetric>>, regions: Vec<&str>) -> anyhow::Result<()> {
    let now = ts_secs(1_800_000_000);
    let metrics: Vec<SmolMetric> = regions
        .into_iter()
        .enumerate()
        .map(|(i, region)| SmolMetric { database_id: (i as i64) + 501, region: smol_str::SmolStr::new(region), collected_at: now, calls: 20 })
        .collect();
    pg.store(&SmolMetric::write(metrics)?).await?;
    Ok(())
}

async fn load_all(pg: &Arc<dyn Backend<SmolMetric>>, filter: Filter) -> anyhow::Result<Vec<SmolMetric>> {
    let mut results = Vec::new();
    for chunk in pg.load(&filter, &[]).await? {
        results.extend(chunk.decompress()?);
    }
    Ok(results)
}

fn ts_secs(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_equality_filter_smostr() -> anyhow::Result<()> {
    setup_table().await?;
    let pg = super::pg::<SmolMetric>();

    store_rows_with_regions(&pg, vec!["us-east", "eu-west", "us-east"]).await?;

    let mut filter = Filter::default();
    filter.region = Some("eu-west".into());

    let results = load_all(&pg, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].region, "eu-west");

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_inclusion_filter_smostr() -> anyhow::Result<()> {
    setup_table().await?;
    let pg = super::pg::<SmolMetric>();

    store_rows_with_regions(&pg, vec!["us-east", "eu-west", "ap-south"]).await?;

    let mut filter = Filter::default();
    filter.region = Some(["us-east", "ap-south"].into());

    let results = load_all(&pg, filter).await?;
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.region == "us-east" || r.region == "ap-south");
    }

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_combined_i64_and_smostr_index_filters() -> anyhow::Result<()> {
    setup_table().await?;
    let pg = super::pg::<SmolMetric>();

    store_rows_with_regions(&pg, vec!["us-east", "eu-west", "us-east"]).await?;

    let mut filter = Filter::default();
    filter.database_id = Some(503.into());
    filter.region = Some("us-east".into());

    let results = load_all(&pg, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].database_id, 503);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_smostr_field_roundtrip_non_index() -> anyhow::Result<()> {
    #[derive(Clone, Debug, PartialEq, PcoStore)]
    #[pco_store(index = [database_id], timestamp = collected_at)]
    pub struct SmolData {
        pub database_id: i64,
        pub label: SmolStr,
        pub collected_at: DateTime<Utc>,
        pub calls: i64,
    }

    type Filter = <SmolData as PcoPack>::Filter;

    let db = &super::DB_POOL.get().await?;
    let pg = super::pg::<SmolData>();
    db.batch_execute(
        "DROP TABLE IF EXISTS smol_datas; CREATE TABLE smol_datas (database_id bigint NOT NULL, label bytea STORAGE EXTERNAL NOT NULL, start_at timestamptz NOT NULL, end_at timestamptz NOT NULL, collected_at bytea STORAGE EXTERNAL NOT NULL, calls bytea STORAGE EXTERNAL NOT NULL);",
    )
    .await?;

    let now = ts_secs(1_800_000_000);
    let metrics: Vec<SmolData> = vec!["alpha", "beta", "gamma"]
        .into_iter()
        .enumerate()
        .map(|(i, label)| SmolData { database_id: (i as i64) + 900, label: smol_str::SmolStr::new(label), collected_at: now, calls: 5 })
        .collect();
    pg.store(&SmolData::write(metrics)?).await?;

    let mut filter = Filter::default();
    filter.database_id = Some(901.into());

    let mut results = Vec::new();
    for chunk in pg.load(&filter, &[]).await? {
        results.extend(chunk.decompress()?);
    }
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].label, "beta");

    Ok(())
}
