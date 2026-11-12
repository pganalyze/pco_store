use anyhow::Context;
use chrono::{DateTime, Utc};
use pco_store::pco_pack::PcoPack;
use pco_store::{Backend, PcoStore, pco_store};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, PcoStore)]
#[pco_store(timestamp = collected_at, index = [database_id, granularity])]
pub struct QueryStat {
    pub database_id: i64,
    pub granularity: i32,
    pub collected_at: DateTime<Utc>,
    pub fingerprint: i64,
}

type Filter = <QueryStat as PcoPack>::Filter;

#[tokio::test]
#[serial_test::serial]
async fn test() -> anyhow::Result<()> {
    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let t2 = t + chrono::Duration::seconds(1);
    let mut filter = Filter::default();
    filter.collected_at = Some((t..=t2).into());
    assert_eq!(filter.range_duration()?, chrono::Duration::seconds(1));
    assert_eq!(filter.range_bounds()?, (t, t2));
    filter.range_shift(chrono::Duration::days(1))?;
    assert_eq!(filter.range_bounds()?, (t + chrono::Duration::days(1), t2 + chrono::Duration::days(1)));
    filter.range_shift(chrono::Duration::days(-2))?;
    assert_eq!(filter.range_bounds()?, (t - chrono::Duration::days(1), t2 - chrono::Duration::days(1)));

    let db = &super::DB_POOL.get().await?;
    let pg = super::pg::<QueryStat>();
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
    pg.store(&QueryStat::write(stats.clone())?).await?;

    // Filtering by a single timestamp
    let actual = load(&pg, Filter::new([5], [60], t..=t)).await?;
    assert_eq!(actual, vec![QueryStat { fingerprint: 1, ..s }, QueryStat { fingerprint: 2, ..s }]);

    // Filtering the whole time range
    let actual = load(&pg, Filter::new([5], [60], t..=t2)).await?;
    assert_eq!(actual, stats);

    // Optional filter via JSON field access - set fingerprint filter in JSON using IndexMut
    {
        let mut filter = Filter::new([5], [60], t..=t2);
        filter["fingerprint"] = serde_json::json!([2]);
        let actual = load(&pg, filter).await?;
        assert_eq!(actual, vec![QueryStat { fingerprint: 2, ..s }]);
    }

    // Delete returns the deleted chunks matching the filter; verify by comparing with load.
    {
        let before_delete = load(&pg, Filter::new([5], [60], t..=t2)).await?;
        assert_eq!(before_delete.len(), 3);
        let deleted = pg.delete(&Filter::new([5], [60], t..=t2)).await?;
        let mut deleted_decompressed = Vec::new();
        for chunk in &deleted {
            deleted_decompressed.extend(chunk.decompress()?);
        }
        deleted_decompressed.sort_by_key(|s| (s.fingerprint, s.collected_at));
        assert_eq!(deleted_decompressed, before_delete);
        let after_delete = load(&pg, Filter::new([5], [60], t..=t2)).await?;
        assert_eq!(after_delete.len(), 0);
    }

    Ok(())
}

async fn load(pg: &Arc<dyn Backend<QueryStat>>, filter: Filter) -> anyhow::Result<Vec<QueryStat>> {
    let mut rows = Vec::new();
    for chunk in pg.load(&filter, &[]).await? {
        rows.extend(chunk.decompress()?);
    }
    rows.sort_by_key(|s| (s.fingerprint, s.collected_at));
    Ok(rows)
}
