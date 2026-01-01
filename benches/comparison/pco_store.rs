use ahash::AHashMap;
use super::*;

#[::pco_store::store(timestamp = collected_at, group_by = [database_id], float_round = 2, table_name = comparison_pco_stores)]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: SystemTime,
    pub collected_secs: i64,
    pub fingerprint: i64,
    pub postgres_role_id: i64,
    pub calls: i64,
    pub rows: i64,
    pub total_time: f64,
    pub io_time: f64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
}

pub async fn store() -> Result<()> {
    let db = &mut DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS comparison_pco_stores;
        CREATE TABLE comparison_pco_stores (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            collected_secs bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL,
            postgres_role_id bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            rows bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL,
            io_time bytea STORAGE EXTERNAL NOT NULL,
            shared_blks_hit bytea STORAGE EXTERNAL NOT NULL,
            shared_blks_read bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON comparison_pco_stores USING btree (database_id);
        CREATE INDEX ON comparison_pco_stores USING btree (end_at, start_at);
    ";
    db.batch_execute(sql).await?;
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM query_stats", &[]).await?.get(0);
    for database_id in database_ids {
        let sql = "
            SELECT start_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
            FROM query_stats WHERE database_id = $1
        ";
        let rows = db.query(sql, &[&database_id]).await?;
        let mut grouped_rows: AHashMap<_, Vec<_>> = AHashMap::new();
        for row in rows {
            let time: DateTime<Utc> = row.get::<_, SystemTime>(0).into();
            let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
            grouped_rows.entry(date).or_default().push(row);
        }
        for rows_ in grouped_rows.into_values() {
            let mut stats = Vec::new();
            for row in rows_ {
                let collected_at: Vec<SystemTime> = row.get(1);
                let collected_secs: Vec<i64> = row.get(2);
                let fingerprint: Vec<i64> = row.get(3);
                let postgres_role_id: Vec<i64> = row.get(4);
                let calls: Vec<i64> = row.get(5);
                let rows: Vec<i64> = row.get(6);
                let total_time: Vec<f64> = row.get(7);
                let io_time: Vec<f64> = row.get(8);
                let shared_blks_hit: Vec<i64> = row.get(9);
                let shared_blks_read: Vec<i64> = row.get(10);
                for (index, collected_at) in collected_at.into_iter().enumerate() {
                    stats.push(QueryStat {
                        database_id,
                        collected_at,
                        collected_secs: collected_secs[index],
                        fingerprint: fingerprint[index],
                        postgres_role_id: postgres_role_id[index],
                        calls: calls[index],
                        rows: rows[index],
                        total_time: total_time[index],
                        io_time: io_time[index],
                        shared_blks_hit: shared_blks_hit[index],
                        shared_blks_read: shared_blks_read[index],
                    });
                }
            }
            CompressedQueryStats::store(db, stats).await?;
        }
    }
    Ok(())
}

pub async fn load() -> Result<()> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM comparison_pco_stores", &[]).await?.get(0);
    let mut stats = Vec::new();
    for group in CompressedQueryStats::load(db, &database_ids, SystemTime::UNIX_EPOCH, SystemTime::now()).await? {
        for stat in group.decompress()? {
            stats.push(stat);
        }
    }
    return Ok(());
}
