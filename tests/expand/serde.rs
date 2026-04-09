#[pco_store::store(group_by = [id, name], timestamp = time)]
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Serde {
    pub id: Uuid,
    pub name: String,
    pub time: DateTime<Utc>,
    pub description: String,
    pub tags: Vec<String>,
    pub nums: Vec<i32>,
    pub map: BTreeMap<String, String>,
    pub json: serde_json::Value,
    pub model: Option<Box<Serde>>,
}
