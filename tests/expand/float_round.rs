#[pco_store::store(group_by = [database_id], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    pub calls: i64,
    pub total_time: f64,
}
