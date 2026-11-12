#[derive(PcoStore)]
#[pco_store(name = query_stats, timestamp = collected_at, index = [database_id])]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub fingerprint: i64,
    pub calls: i64,
}
