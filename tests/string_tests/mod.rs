use super::DB_POOL;
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[pco_store::store(timestamp = collected_at, group_by = [server_id], float_round = 2)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemStorageStat {
    pub server_id: Uuid,
    pub collected_at: DateTime<Utc>,
    pub mountpoint: String,
    pub bytes: i64,
    pub latency: f64,
}

#[tokio::test]
#[serial_test::serial]
async fn test() -> anyhow::Result<()> {
    let db = &DB_POOL.get().await?;
    let sql = "
        DROP TABLE IF EXISTS system_storage_stats;
        CREATE TABLE system_storage_stats (
            server_id uuid NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            mountpoint bytea STORAGE EXTERNAL NOT NULL,
            bytes bytea STORAGE EXTERNAL NOT NULL,
            latency bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON system_storage_stats USING btree (server_id, end_at, start_at);
    ";
    db.batch_execute(sql).await?;

    let server_id = Uuid::default();
    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let t2 = t + Duration::seconds(1);
    let s = SystemStorageStat { server_id, collected_at: t, ..Default::default() };
    let s1 = SystemStorageStat { mountpoint: "/".into(), bytes: 1, latency: 1.0, ..s };
    let s2 = SystemStorageStat { mountpoint: "/other".into(), bytes: 2, latency: 2.0, ..s };
    let s3 = SystemStorageStat { mountpoint: "/".into(), bytes: 3, latency: 3.0, collected_at: t2, ..s };
    let stats = vec![s1.clone(), s2.clone(), s3.clone()];
    CompressedSystemStorageStats::store(db, stats.clone()).await?;

    // Filtering by a single timestamp
    let actual = load(db, Filter::new(&[server_id], t..=t)).await?;
    assert_eq!(actual, vec![s1.clone(), s2.clone()]);

    // Filtering the whole time range
    let actual = load(db, Filter::new(&[server_id], t..=t2)).await?;
    assert_eq!(actual, stats);

    // Filtering by compressed String field
    let mut filter = Filter::new(&[server_id], t..=t2);
    filter.mountpoint = vec!["/other".into()];
    let actual = load(db, filter).await?;
    assert_eq!(actual, vec![s2.clone()]);

    Ok(())
}

async fn load(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<SystemStorageStat>> {
    let mut rows = Vec::new();
    for group in CompressedSystemStorageStats::load(db, filter, ()).await? {
        rows.extend(group.decompress()?);
    }
    rows.sort_by_key(|s| (s.bytes, s.collected_at));
    Ok(rows)
}
