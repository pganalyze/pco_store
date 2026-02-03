use anyhow::Context;
use chrono::{DateTime, Duration, NaiveDate, Utc};

fn ymd_hms_micros(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, micros: u32) -> Option<DateTime<Utc>> {
    Some(NaiveDate::from_ymd_opt(year, month, day)?.and_hms_micro_opt(hour, min, sec, micros)?.and_utc())
}

#[pco_store::store(timestamp = collected_at, group_by = [database_id, granularity])]
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
    let start = ymd_hms_micros(2026, 01, 01, 1, 01, 02, 345_678).unwrap();
    let end = ymd_hms_micros(2026, 01, 01, 5, 03, 04, 567_890).unwrap();
    let filter = Filter::new(&[5], &[60], start..=end);
    assert_eq!(filter, Filter { database_id: vec![5], granularity: vec![60], collected_at: Some(start..=end), fingerprint: vec![] });

    // Nanosecond precision is removed
    let s = start + Duration::nanoseconds(123);
    let e = end + Duration::nanoseconds(456);
    assert_ne!(start, s);
    let mut filter = Filter::new(&[5], &[60], s..=e);
    assert_eq!(filter.collected_at, Some(s..=e));
    filter.range_truncate()?;
    assert_eq!(filter.collected_at, Some(start..=end));

    // Convenience functions
    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let t2 = t + Duration::seconds(1);
    let mut filter = Filter::new(&[], &[], t..=t2);
    assert_eq!(filter.range_duration()?, Duration::seconds(1));
    assert_eq!(filter.range_bounds()?, (t, t2));
    filter.range_shift(Duration::days(1))?;
    assert_eq!(filter.range_bounds()?, (t + Duration::days(1), t2 + Duration::days(1)));
    filter.range_shift(Duration::days(-2))?;
    assert_eq!(filter.range_bounds()?, (t - Duration::days(1), t2 - Duration::days(1)));

    // Deserialization
    let filter: Filter =
        serde_json::from_str(r#"{"database_id": [1], "collected_at": ["2026-01-01T01:01:02.345678Z", "2026-01-01T05:03:04.567890Z"]}"#)?;
    assert_eq!(filter.database_id, vec![1]);
    assert_eq!(filter.collected_at, Some(start..=end));
    let filter: Filter = serde_json::from_str(r#"{"database_id": 1, "collected_at": "2026-01-01T01:01:02.345678Z"}"#)?;
    assert_eq!(filter.database_id, vec![1]);
    assert_eq!(filter.collected_at, Some(start..=start));
    let filter: Filter = serde_json::from_str(r#"{"collected_at": ["2026-01-01T01:01:02.345678Z"]}"#)?;
    assert_eq!(filter.collected_at, Some(start..=start));
    let filter: Filter = serde_json::from_str(r#"{"collected_at": []}"#)?;
    assert_eq!(filter.collected_at, None);
    let filter: Filter = serde_json::from_str(r#"{"collected_at": null}"#)?;
    assert_eq!(filter.collected_at, None);

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
    let actual = load(db, Filter::new(&[5], &[60], t..=t)).await?;
    assert_eq!(actual, vec![QueryStat { fingerprint: 1, ..s }, QueryStat { fingerprint: 2, ..s },]);

    // Filtering the whole time range
    let actual = load(db, Filter::new(&[5], &[60], t..=t2)).await?;
    assert_eq!(actual, stats);

    // Optional filter
    let mut filter = Filter::new(&[5], &[60], t..=t2);
    filter.fingerprint = vec![2];
    let actual = load(db, filter).await?;
    assert_eq!(actual, vec![QueryStat { fingerprint: 2, ..s }]);

    // Optional filter is not applied to deleted rows
    let mut filter = Filter::new(&[5], &[60], t..=t2);
    filter.fingerprint = vec![2];
    let actual = delete(db, filter).await?;
    assert_eq!(actual, stats);

    Ok(())
}

async fn load(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for group in CompressedQueryStats::load(db, filter, ()).await? {
        rows.extend(group.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}

async fn delete(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for group in CompressedQueryStats::delete(db, filter, ()).await? {
        rows.extend(group.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}
