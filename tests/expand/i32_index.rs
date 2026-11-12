#[pco_store::store(timestamp = collected_at, index = [database_id, granularity])]
pub struct QueryStat {
    pub database_id: i64,
    pub granularity: i32,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub calls: i64,
}
