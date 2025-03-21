#[pco_store::store(timestamp = collected_at, group_by = [database_id], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: SystemTime,
    pub collected_secs: i64,
    pub fingerprint: i64,
    pub postgres_role_id: i64,
    pub toplevel: bool,
    pub calls: i64,
    pub rows: i64,
    pub total_time: f64,
    pub io_time: f64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
}
