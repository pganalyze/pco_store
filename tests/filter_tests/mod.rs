use anyhow::Context;
use chrono::{DateTime, Duration, Utc};

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
    // Convenience functions on Filter for timestamp manipulation (provided by pco_pack)
    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let t2 = t + Duration::seconds(1);
    let mut filter = Filter::default();
    filter.collected_at = Some((t..=t2).into());
    assert_eq!(filter.range_duration()?, Duration::seconds(1));
    assert_eq!(filter.range_bounds()?, (t, t2));
    filter.range_shift(Duration::days(1))?;
    assert_eq!(filter.range_bounds()?, (t + Duration::days(1), t2 + Duration::days(1)));
    filter.range_shift(Duration::days(-2))?;
    assert_eq!(filter.range_bounds()?, (t - Duration::days(1), t2 - Duration::days(1)));

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

    let mut stats = Vec::new();
    let s = QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 0 };
    stats.push(QueryStat { fingerprint: 1, ..s });
    stats.push(QueryStat { fingerprint: 2, ..s });
    stats.push(QueryStat { fingerprint: 3, collected_at: t2, ..s });
    CompressedQueryStats::store(db, stats.clone()).await?;

    // Filtering by a single timestamp
    let actual = load(db, Filter::new([5], [60], t..=t)).await?;
    assert_eq!(actual, vec![QueryStat { fingerprint: 1, ..s }, QueryStat { fingerprint: 2, ..s }]);

    // Filtering the whole time range
    let actual = load(db, Filter::new([5], [60], t..=t2)).await?;
    assert_eq!(actual, stats);

    // Optional filter via JSON field access - set fingerprint filter in JSON using IndexMut
    {
        let mut filter = Filter::new([5], [60], t..=t2);
        filter["fingerprint"] = serde_json::json!([2]);
        let actual = load(db, filter).await?;
        assert_eq!(actual, vec![QueryStat { fingerprint: 2, ..s }]);
    }

    // Delete returns the deleted chunks matching the filter; verify by comparing with load.
    {
        let before_delete = load(db, Filter::new([5], [60], t..=t2)).await?;
        assert_eq!(before_delete.len(), 3);
        let deleted = CompressedQueryStats::delete(db, Filter::new([5], [60], t..=t2), &[]).await?;
        let mut deleted_decompressed = Vec::new();
        for chunk in &deleted {
            deleted_decompressed.extend(chunk.decompress()?);
        }
        deleted_decompressed.sort_by_key(|s| (s.fingerprint, s.collected_at));
        assert_eq!(deleted_decompressed, before_delete);
        let after_delete = load(db, Filter::new([5], [60], t..=t2)).await?;
        assert_eq!(after_delete.len(), 0);
    }

    Ok(())
}

async fn load(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for chunk in CompressedQueryStats::load(db, filter, &[]).await? {
        rows.extend(chunk.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}
