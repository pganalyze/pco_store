use anyhow::Context;
use chrono::{DateTime, Utc};

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
    assert_eq!(Fields::default().select(), "database_id, granularity, collected_at, fingerprint");
    assert_eq!(Fields::required().select(), "database_id, granularity, collected_at");

    assert_eq!(Fields::default(), ().try_into().unwrap());
    assert_eq!(Fields::required(), vec![].try_into().unwrap());
    assert_eq!(Fields::default(), Fields::try_from(&["fingerprint"]).unwrap());

    assert_eq!(Fields::default(), serde_json::from_str(r#"null"#)?);
    assert_eq!(Fields::required(), serde_json::from_str(r#"[]"#)?);
    assert_eq!(Fields::default(), serde_json::from_str(r#"["fingerprint"]"#)?);
    assert!(serde_json::from_str::<Fields>(r#"["other"]"#).is_err());

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
    let filter = Filter::new(&[5], &[60], t..=t);
    let s = QueryStat { database_id: 5, granularity: 60, collected_at: t, fingerprint: 0 };
    let mut stats = Vec::new();
    stats.push(QueryStat { fingerprint: 1, ..s });
    stats.push(QueryStat { fingerprint: 2, ..s });
    CompressedQueryStats::store(db, stats.clone()).await?;

    // Including all fields
    let full = vec![QueryStat { fingerprint: 1, ..s }, QueryStat { fingerprint: 2, ..s }];
    assert_eq!(full, load(db, filter.clone(), Fields::default()).await?);
    assert_eq!(full, load(db, filter.clone(), Fields::new(&["fingerprint"])?).await?);
    assert_eq!(full, load(db, filter.clone(), ()).await?);
    assert_eq!(full, load(db, filter.clone(), &["fingerprint"]).await?);

    // Skipping some fields
    let partial = vec![QueryStat { fingerprint: 0, ..s }, QueryStat { fingerprint: 0, ..s }];
    assert_eq!(partial, load(db, filter.clone(), Fields::new(&[])?).await?);
    assert_eq!(partial, load(db, filter.clone(), &[]).await?);

    // Error cases
    assert!(Fields::new(&["other"]).is_err());
    assert!(load(db, filter.clone(), &["other"]).await.is_err());

    // Optional filters are automatically included in the fields to be loaded
    let mut f = filter.clone();
    f.fingerprint = vec![2];
    assert_eq!(vec![QueryStat { fingerprint: 2, ..s }], load(db, f.clone(), &[]).await?);

    // Fields can be skipped skipped when deleting rows
    assert_eq!(partial, delete(db, filter.clone(), &[]).await?);
    CompressedQueryStats::store(db, stats).await?;
    assert_eq!(full, delete(db, filter.clone(), ()).await?);

    Ok(())
}

async fn load(db: &deadpool_postgres::Client, filter: Filter, fields: impl TryInto<Fields>) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for group in CompressedQueryStats::load(db, filter, fields).await? {
        rows.extend(group.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}

async fn delete(db: &deadpool_postgres::Client, filter: Filter, fields: impl TryInto<Fields>) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for group in CompressedQueryStats::delete(db, filter, fields).await? {
        rows.extend(group.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}
