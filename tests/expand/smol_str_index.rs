#[pco_store::store(timestamp = collected_at, index = [database_id, region])]
pub struct QueryStat {
    pub database_id: i64,
    pub region: smol_str::SmolStr,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub fingerprint: i64,
    pub calls: i64,
}
