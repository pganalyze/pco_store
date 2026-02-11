use chrono::{DateTime, Utc};
use uuid::Uuid;

#[pco_store::store(timestamp = collected_at, group_by = [server_id, granularity], float_round = 2)]
pub struct SystemStorageStat {
    pub server_id: Uuid,
    pub granularity: i32,
    pub mountpoint: String,
    pub collected_at: DateTime<Utc>,
    pub bytes_available: i64,
    pub bytes_total: i64,
    pub queue_depth: i64,
    pub read_latency: f64,
    pub write_latency: f64,
}
