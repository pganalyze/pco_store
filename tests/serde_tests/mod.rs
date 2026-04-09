use super::DB_POOL;
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn string() -> anyhow::Result<()> {
    let id = Uuid::default();
    let t = DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).context("out of range")?;
    let t2 = t + Duration::seconds(1);
    let t3 = t2 + Duration::seconds(1);

    {
        #[pco_store::store(group_by = [id, name], timestamp = time)]
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct Model {
            pub id: Uuid,
            pub name: String,
            pub time: DateTime<Utc>,
            pub description: String,
            pub tags: Vec<String>,
            pub nums: Vec<i32>,
            pub map: BTreeMap<String, String>,
            pub json: serde_json::Value,
            pub model: Option<Box<Model>>,
        }
        async fn load(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<Model>> {
            let mut rows = Vec::new();
            for group in CompressedModels::load(db, filter, ()).await? {
                rows.extend(group.decompress()?);
            }
            rows.sort_by_key(|s| (s.id, s.name.clone(), s.time));
            Ok(rows)
        }
        let db = &DB_POOL.get().await?;
        let sql = "
            DROP TABLE IF EXISTS models;
            CREATE TABLE models (
                id uuid NOT NULL,
                name text NOT NULL,
                start_at timestamptz NOT NULL,
                end_at timestamptz NOT NULL,
                time bytea STORAGE EXTERNAL NOT NULL,
                description bytea STORAGE EXTERNAL NOT NULL,
                tags bytea STORAGE EXTERNAL NOT NULL,
                nums bytea STORAGE EXTERNAL NOT NULL,
                map bytea STORAGE EXTERNAL NOT NULL,
                json bytea STORAGE EXTERNAL NOT NULL,
                model bytea STORAGE EXTERNAL NOT NULL
            );
            CREATE INDEX ON models USING btree (id, name, start_at, end_at);
        ";
        db.batch_execute(sql).await?;

        let a =
            Model { id, name: "a".into(), description: "desc".into(), tags: vec!["x".into(), "y".into()], nums: vec![8, 9], ..Default::default() };
        let a1 = Model { id, time: t, ..a.clone() };
        let a2 = Model {
            id,
            time: t2,
            tags: vec!["x".into(), "y".into(), "z".into()],
            nums: vec![8, 9, 10],
            description: "other".into(),
            map: [("k".into(), "v".into())].into(),
            json: json!(null),
            model: Some(a1.clone().into()),
            ..a.clone()
        };
        let data = vec![a1.clone(), a2.clone()];
        CompressedModels::store(db, data.clone()).await?;

        // Filtering by a single timestamp
        let actual = load(db, Filter::new(&[id], &["a".into()], t..=t)).await?;
        assert_eq!(actual, vec![a1.clone()]);

        // Filtering the whole time range
        let actual = load(db, Filter::new(&[id], &["a".into()], t..=t2)).await?;
        assert_eq!(actual, data);

        // Filtering by compressed String field
        let mut filter = Filter::new(&[id], &["a".into()], t..=t2);
        filter.description = vec!["other".into()];
        let actual = load(db, filter).await?;
        assert_eq!(actual, vec![a2.clone()]);
    }

    // Adding a new column to the table doesn't break decompression of existing rows
    {
        #[pco_store::store(group_by = [id, name], timestamp = time)]
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct Model {
            pub id: Uuid,
            pub name: String,
            pub time: DateTime<Utc>,
            pub description: String,
            pub tags: Vec<String>,
            pub nums: Vec<i32>,
            pub map: BTreeMap<String, String>,
            pub json: serde_json::Value,
            pub model: Option<Box<Model>>,
            #[serde(default)]
            pub new: bool,
        }
        async fn load(db: &deadpool_postgres::Client, filter: Filter) -> anyhow::Result<Vec<Model>> {
            let mut rows = Vec::new();
            for group in CompressedModels::load(db, filter, ()).await? {
                rows.extend(group.decompress()?);
            }
            rows.sort_by_key(|s| (s.id, s.name.clone(), s.time));
            Ok(rows)
        }
        let db = &DB_POOL.get().await?;
        db.execute("ALTER TABLE models ADD COLUMN new bytea STORAGE EXTERNAL DEFAULT '' NOT NULL", &[]).await?;

        let a = Model { id, name: "a".into(), time: t3, new: true, ..Default::default() };
        CompressedModels::store(db, vec![a.clone()]).await?;

        // Filtering by a single timestamp
        let actual = load(db, Filter::new(&[id], &["a".into()], t3..=t3)).await?;
        assert_eq!(actual, vec![a.clone()]);

        // Filtering the whole time range
        let actual = load(db, Filter::new(&[id], &["a".into()], t..=t3)).await?;
        assert_eq!(actual.len(), 3);

        // Filtering by new field
        let mut filter = Filter::new(&[id], &["a".into()], t..=t3);
        filter.new = vec![true];
        let actual = load(db, filter).await?;
        assert_eq!(actual, vec![a.clone()]);
    }

    Ok(())
}
