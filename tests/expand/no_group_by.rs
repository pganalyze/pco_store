#[pco_store::store]
pub struct QueryStat {
    pub database_id: i64,
    pub calls: i64,
    pub total_time: f64,
}
