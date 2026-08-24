use anyhow::Context;
use chrono::{DateTime, Utc};

#[pco_store::store(timestamp = collected_at, index = [database_id, granularity])]
#[derive(Clone, Debug, PartialEq)]
pub struct QueryStat {
    pub database_id: i64,
    pub granularity: i32,
    pub collected_at: DateTime<Utc>,
    pub fingerprint: i64,
}

#[tokio::test]
#[serial_test::serial]
async fn test() -> anyhow::Result<()> {
    let db = &super::DB_POOL.get().await?;
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            granularity int NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
    ";
    db.batch_execute(sql).await?;

    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let s = QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 0 };
    let mut stats = Vec::new();
    stats.push(QueryStat { fingerprint: 1, ..s });
    stats.push(QueryStat { fingerprint: 2, ..s });
    CompressedQueryStats::store(db, stats.clone()).await?;

    // Including all fields (empty slice means "all fields")
    let full = vec![QueryStat { fingerprint: 1, ..s }, QueryStat { fingerprint: 2, ..s }];
    assert_eq!(full, load(db, Filter::new([5], [60], t..=t), &[]).await?);

    // Requesting specific field by name
    assert_eq!(full, load(db, Filter::new([5], [60], t..=t), &["fingerprint"]).await?);

    // Error case: unknown field name is rejected by pco_pack
    assert!(load(db, Filter::new([5], [60], t..=t), &["other"]).await.is_err());

    // Optional filters are automatically included in the fields to be loaded
    let mut f = Filter::new([5], [60], t..=t);
    f["fingerprint"] = serde_json::json!([2]);
    assert_eq!(vec![QueryStat { fingerprint: 2, ..s }], load(db, f, &[]).await?);

    // Delete returns the deleted chunks matching the filter; verify by comparing decompressed rows.
    let before_delete = load(db, Filter::new([5], [60], t..=t), &[]).await?;
    assert_eq!(before_delete.len(), 2);
    let deleted_chunks = CompressedQueryStats::delete(db, Filter::new([5], [60], t..=t), &[]).await?;
    let mut deleted = Vec::new();
    for chunk in deleted_chunks {
        deleted.extend(chunk.decompress()?);
    }
    deleted.sort_by_key(|s| (s.fingerprint, s.collected_at));
    assert_eq!(deleted.len(), 2);
    assert_eq!(deleted, before_delete);
    let after_delete = load(db, Filter::new([5], [60], t..=t), &[]).await?;
    assert_eq!(after_delete.len(), 0);

    Ok(())
}

/// Test that loading with specific fields only requests those fields from SQL.
#[tokio::test]
#[serial_test::serial]
async fn test_selective_field_loading() -> anyhow::Result<()> {
    let db = &super::DB_POOL.get().await?;
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            granularity int NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
    ";
    db.batch_execute(sql).await?;

    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let s = QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 42 };
    CompressedQueryStats::store(db, vec![s]).await?;

    // Load only fingerprint field
    let chunks = CompressedQueryStats::load(db, Filter::new([5], [60], t..=t), &["fingerprint"]).await?;
    assert_eq!(chunks.len(), 1);
    let chunk = &chunks[0];

    // Verify the chunk stores the resolved fields (fingerprint + timestamp fields from filter)
    let fields_set: std::collections::HashSet<&str> = chunk.fields.iter().map(|s| s.as_str()).collect();
    assert!(fields_set.contains("fingerprint"), "should include requested fingerprint field");
    // Timestamp filter should include start_at/end_at
    assert!(fields_set.contains("start_at"), "timestamp filter should include start_at");
    assert!(fields_set.contains("end_at"), "timestamp filter should include end_at");

    // Decompression should still work with the loaded fields
    let decompressed = chunk.decompress()?;
    assert_eq!(decompressed.len(), 1);
    assert_eq!(decompressed[0].fingerprint, 42);

    // Load with empty fields (all fields)
    let chunks = CompressedQueryStats::load(db, Filter::new([5], [60], t..=t), &[]).await?;
    assert_eq!(chunks.len(), 1);
    let decompressed = chunks[0].decompress()?;
    assert_eq!(decompressed[0].database_id, 5);
    assert_eq!(decompressed[0].granularity, 60);
    assert_eq!(decompressed[0].fingerprint, 42);

    Ok(())
}

/// Test that fields used in optional filters are automatically included.
#[tokio::test]
#[serial_test::serial]
async fn test_filter_fields_auto_included() -> anyhow::Result<()> {
    let db = &super::DB_POOL.get().await?;
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            granularity int NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
    ";
    db.batch_execute(sql).await?;

    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    CompressedQueryStats::store(
        db,
        vec![
            QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 1 },
            QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 2 },
        ],
    )
    .await?;

    // Filter on fingerprint but request no explicit fields - fingerprint should be auto-included
    let mut f = Filter::new([5], [60], t..=t);
    f["fingerprint"] = serde_json::json!([2]);
    let chunks = CompressedQueryStats::load(db, f.clone(), &[]).await?;
    assert_eq!(chunks.len(), 1);

    // Verify fingerprint field is in the resolved fields
    let fields_set: std::collections::HashSet<&str> = chunks[0].fields.iter().map(|s| s.as_str()).collect();
    assert!(fields_set.contains("fingerprint"), "filter field should be auto-included");

    // Verify only matching row is returned
    let decompressed = chunks[0].decompress()?;
    assert_eq!(decompressed.len(), 1);
    assert_eq!(decompressed[0].fingerprint, 2);

    Ok(())
}

/// Test loading without timestamp fields.
#[tokio::test]
#[serial_test::serial]
async fn test_no_timestamp_fields() -> anyhow::Result<()> {
    #[pco_store::store(index = [database_id])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct NoTsStat {
        pub database_id: i64,
        pub calls: i64,
        pub total_time: f64,
    }

    let db = &super::DB_POOL.get().await?;
    let sql = "
        DROP TABLE IF EXISTS no_ts_stats;
        CREATE TABLE no_ts_stats (
            database_id bigint NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON no_ts_stats USING btree (database_id);
    ";
    db.batch_execute(sql).await?;

    CompressedNoTsStats::store(
        db,
        vec![NoTsStat { database_id: 1, calls: 10, total_time: 1.5 }, NoTsStat { database_id: 1, calls: 20, total_time: 2.5 }],
    )
    .await?;

    // Load only calls field
    let chunks = CompressedNoTsStats::load(db, Filter::new([1]), &["calls"]).await?;
    assert_eq!(chunks.len(), 1);
    let decompressed = chunks[0].decompress()?;
    assert_eq!(decompressed.len(), 2);
    assert_eq!(decompressed[0].calls, 10);
    assert_eq!(decompressed[1].calls, 20);

    // Load all fields
    let chunks = CompressedNoTsStats::load(db, Filter::new([1]), &[]).await?;
    let decompressed = chunks[0].decompress()?;
    assert_eq!(decompressed[0].total_time, 1.5);
    assert_eq!(decompressed[1].total_time, 2.5);

    Ok(())
}

async fn load(db: &deadpool_postgres::Client, filter: Filter, fields: &[&str]) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for chunk in CompressedQueryStats::load(db, filter, fields).await? {
        rows.extend(chunk.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}
