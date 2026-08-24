//! Integration tests for direct field access on generated Filter structs with #[store(index = [...])]
//!
//! Tests that filters correctly reduce the result set when used with pco_store.

use chrono::{DateTime, Utc};

/// Struct with multiple index fields of different types to exercise all filter variant handlers.
#[pco_store::store(index = [database_id, category], timestamp = collected_at)]
#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    pub database_id: i64,
    pub category: String,
    pub collected_at: DateTime<Utc>,
    pub latency_us: f64,
    pub calls: i64,
}

// Create a deterministic UTC time from seconds-since-epoch value.
fn ts_secs(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_equality_filter_i64() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows(db, vec![10, 20, 20]).await?;

    // Direct field access with .into() should generate "database_id = $1"
    let mut filter = Filter::default();
    filter.database_id = Some(20.into());

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 2, "Expected only database_id=20 rows");
    for r in &results {
        assert_eq!(r.database_id, 20);
    }

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_inclusion_filter_i64() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows(db, vec![10, 20, 30]).await?;

    // Direct field access with Vec.into() should generate "database_id = ANY($N)"
    let mut filter = Filter::default();
    filter.database_id = Some(vec![10, 30].into());

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 2);
    let ids: Vec<i64> = results.iter().map(|r| r.database_id).collect();
    assert!(ids.contains(&10));
    assert!(ids.contains(&30));

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_range_filter_i64() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows(db, vec![5, 10, 20]).await?;

    // Range.into() generates "database_id >= $min AND database_id <= $max"
    let mut filter = Filter::default();
    filter.database_id = Some((8i64..=15).into());

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].database_id, 10);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_equality_filter_string() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows_with_cats(db, vec!["a", "b", "a"]).await?;

    let mut filter = Filter::default();
    filter.category = Some("b".into());

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].category, "b");

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_direct_field_inclusion_filter_string() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows_with_cats(db, vec!["x", "y", "z"]).await?;

    let mut filter = Filter::default();
    filter.category = Some(["x", "z"].into());

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.category == "x" || r.category == "z");
    }

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_indexmut_fallback_array_inclusion() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows(db, vec![10, 20, 30]).await?;

    let mut filter = Filter::default();
    filter["database_id"] = serde_json::json!([10, 30]);

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.database_id == 10 || r.database_id == 30);
    }

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_indexmut_fallback_single_equality() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows(db, vec![10, 20]).await?;

    // IndexMut-style access with single value should produce equality match
    let mut filter = Filter::default();
    filter["database_id"] = serde_json::json!(10);

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].database_id, 10);

    Ok(())
}

async fn setup_table() -> anyhow::Result<()> {
    let db = &super::DB_POOL.get().await?;
    db.batch_execute(
        "DROP TABLE IF EXISTS metrics;
         CREATE TABLE metrics (
             database_id bigint NOT NULL,
             category text NOT NULL,
             start_at timestamptz NOT NULL,
             end_at timestamptz NOT NULL,
             collected_at bytea STORAGE EXTERNAL NOT NULL,
             latency_us bytea STORAGE EXTERNAL NOT NULL,
             calls bytea STORAGE EXTERNAL NOT NULL
         );",
    )
    .await?;
    Ok(())
}

/// Store rows with given database_ids (default category)
async fn store_rows(db: &deadpool_postgres::Client, ids: Vec<i64>) -> anyhow::Result<()> {
    let now = ts_secs(1_800_000_000);
    let metrics: Vec<Metric> = ids
        .into_iter()
        .map(|db_id| Metric { database_id: db_id, category: "default".into(), collected_at: now, latency_us: 5.0, calls: 100 })
        .collect();
    CompressedMetrics::store(db, metrics).await?;
    Ok(())
}

/// Store rows with given categories (sequential database_ids)
async fn store_rows_with_cats(db: &deadpool_postgres::Client, cats: Vec<&str>) -> anyhow::Result<()> {
    let now = ts_secs(1_800_000_000);
    let metrics: Vec<Metric> = cats
        .into_iter()
        .enumerate()
        .map(|(i, cat)| Metric { database_id: (i as i64) + 100, category: cat.to_string(), collected_at: now, latency_us: 5.0, calls: 100 })
        .collect();
    CompressedMetrics::store(db, metrics).await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_timestamp_direct_field_range() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    // Store rows at different times (all with same database_id)
    store_rows_at_times(db).await?;

    // Set timestamp via direct DateTimeFilter field.
    let mut filter = Filter::default();
    filter.database_id = Some(1.into());
    // Direct Range DateTimeFilter, matching t2 only.
    filter.collected_at = Some(pco_pack::DateTimeFilter::Range { start: ts_secs(2_000_000), end: ts_secs(2_999_999) });

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].collected_at.timestamp(), 2_000_000);

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_timestamp_direct_field_range_narrow() -> anyhow::Result<()> {
    setup_table().await?;
    let db = &super::DB_POOL.get().await?;

    store_rows_at_times(db).await?;

    // Direct Range DateTimeFilter, matching t2 and t3 only.
    let mut filter = Filter::default();
    filter.database_id = Some(1.into());
    filter.collected_at = Some(pco_pack::DateTimeFilter::Range { start: ts_secs(2_000_000), end: ts_secs(3_000_000) });

    let results = load_all(db, filter).await?;
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.collected_at >= ts_secs(2_000_000) && r.collected_at <= ts_secs(3_000_000));
    }

    Ok(())
}

/// Store rows at distinct timestamps (all same database_id)
async fn store_rows_at_times(db: &deadpool_postgres::Client) -> anyhow::Result<()> {
    let metrics = vec![
        Metric { database_id: 1, category: "default".into(), collected_at: ts_secs(1_000_000), latency_us: 5.0, calls: 100 },
        Metric { database_id: 1, category: "default".into(), collected_at: ts_secs(2_000_000), latency_us: 5.0, calls: 100 },
        Metric { database_id: 1, category: "default".into(), collected_at: ts_secs(3_000_000), latency_us: 5.0, calls: 100 },
    ];
    CompressedMetrics::store(db, metrics).await?;
    Ok(())
}

async fn load_all(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<Metric>> {
    let mut results = Vec::new();
    for chunk in CompressedMetrics::load(db, filter, &[]).await? {
        results.extend(chunk.decompress()?);
    }
    Ok(results)
}
