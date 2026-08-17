#[pco_store::store(index = [database_id], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    pub toplevel: bool,
    pub calls: i64,
}
