use super::*;

#[::pco_store::store(timestamp = collected_at, group_by = [database_id], float_round = 2, table_name = synthetic_pco_stores)]
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


pub async fn store() -> Result<Duration> {
    let db = &mut DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS synthetic_pco_stores;
        CREATE TABLE synthetic_pco_stores (
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
        CREATE INDEX ON synthetic_pco_stores USING btree (database_id, end_at, start_at);
    ";
    db.batch_execute(sql).await?;

    let mut stats = Vec::new();
    for db_id in 0..100 {
        for i in 0..100_000 {
            stats.push(QueryStat {
                database_id: db_id,
                collected_at: SystemTime::now(),
                collected_secs: 10,
                fingerprint: i % 500,
                postgres_role_id: i % 1_000,
                calls: 100 + i,
                rows: 10 + i,
                total_time: 1234.0,
                io_time: 12345.0,
                shared_blks_hit: 10 + i,
                shared_blks_read: 20 + i,
            });
        }
    }

    let start = Instant::now();
    CompressedQueryStats::store(db, stats).await?;

    Ok(start.elapsed())
}

pub async fn load() -> Result<Duration> {
    let db = &DB_POOL.get().await.unwrap();
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM synthetic_pco_stores", &[]).await?.get(0);
    let mut stats = Vec::new();
    let filter = Filter::new(&database_ids, SystemTime::UNIX_EPOCH..=SystemTime::now());

    // This assumes the stats.push() call takes negligible time.
    let start = Instant::now();
    for group in CompressedQueryStats::load(db, filter, ()).await? {
        for stat in group.decompress()? {
            stats.push(stat);
        }
    }
    return Ok(start.elapsed());
}
