use pco_pack::PcoPack;
#[derive(PcoPack)]
#[pco_pack(timestamp = time, index = [id, name])]
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
/// Type alias for the compressed chunk representation.
pub type Chunk = <Serde as PcoPack>::Chunk;
/// Typed filter struct provided by pco_pack.
pub type Filter = <Serde as PcoPack>::Filter;
/// A single row of compressed data that can be decompressed on demand.
pub struct CompressedSerdes {
    chunk: Chunk,
    filter: Filter,
    fields: Vec<String>,
}
impl std::ops::Deref for CompressedSerdes {
    type Target = Chunk;
    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}
impl CompressedSerdes {
    /// Decompresses this chunk using the filter and fields used to load it.
    pub fn decompress(&self) -> anyhow::Result<Vec<Serde>> {
        let fields: Vec<&str> = self.fields.iter().map(|s| s.as_str()).collect();
        <Serde as PcoPack>::filter(
            std::slice::from_ref(&self.chunk),
            self.filter.clone(),
            &fields,
        )
    }
    /// Loads data for the specified filters.
    pub async fn load(
        db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        filter: Filter,
        fields: &[&str],
    ) -> anyhow::Result<Vec<Self>> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        if let Some(f) = filter.id.as_ref() {
            match f {
                pco_pack::UuidFilter::Equal(v) => {
                    where_clauses.push(format!("{} = ${}", "id", params.len() + 1));
                    params.push(v);
                }
                pco_pack::UuidFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(format!("{} = ANY(${})", "id", params.len() + 1));
                        params.push(vals);
                    }
                }
            }
        }
        if let Some(f) = filter.name.as_ref() {
            match f {
                pco_pack::StringFilter::Equal(v) => {
                    where_clauses.push(format!("{} = ${}", "name", params.len() + 1));
                    params.push(v);
                }
                pco_pack::StringFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(format!("{} = ANY(${})", "name", params.len() + 1));
                        params.push(vals);
                    }
                }
            }
        }
        if let Some(f) = filter.time.as_ref() {
            match f {
                pco_pack::DateTimeFilter::Equal(v) => {
                    where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                    params.push(v);
                    where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                    params.push(v);
                }
                pco_pack::DateTimeFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(vals.first().unwrap());
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(vals.last().unwrap());
                    }
                }
                pco_pack::DateTimeFilter::Range { start, end } => {
                    where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                    params.push(start);
                    where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                    params.push(end);
                }
            }
        }
        let sql_where = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let filter_json: serde_json::Value = filter.clone().try_into()?;
        let fields = <Serde as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &[
            "id",
            "name",
            "start_at",
            "end_at",
            "time",
            "description",
            "tags",
            "nums",
            "map",
            "json",
            "model",
        ];
        let select_columns: Vec<_> = known
            .iter()
            .filter(|c| fields.contains(*c))
            .copied()
            .collect();
        let query_sql = format!(
            "SELECT {} FROM {} {}", select_columns.join(", "), "serdes", sql_where
        );
        let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
        let mut chunks: Vec<Self> = Vec::new();
        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
            let mut index = 0;
            chunks
                .push(Self {
                    chunk: Chunk {
                        id: if fields.contains(&"id") {
                            let v = row.get(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        name: if fields.contains(&"name") {
                            let v = row.get(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        start_at: if fields.contains(&"start_at") {
                            let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        end_at: if fields.contains(&"end_at") {
                            let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        time: if fields.contains(&"time") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        description: if fields.contains(&"description") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        tags: if fields.contains(&"tags") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        nums: if fields.contains(&"nums") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        map: if fields.contains(&"map") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        json: if fields.contains(&"json") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        model: if fields.contains(&"model") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                    },
                    filter: filter.clone(),
                    fields: fields_vec.clone(),
                });
        }
        Ok(chunks)
    }
    /// Deletes data matching the specified filters and returns the deleted rows.
    pub async fn delete(
        db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        filter: Filter,
        fields: &[&str],
    ) -> anyhow::Result<Vec<Self>> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        if let Some(f) = filter.id.as_ref() {
            match f {
                pco_pack::UuidFilter::Equal(v) => {
                    where_clauses.push(format!("{} = ${}", "id", params.len() + 1));
                    params.push(v);
                }
                pco_pack::UuidFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(format!("{} = ANY(${})", "id", params.len() + 1));
                        params.push(vals);
                    }
                }
            }
        }
        if let Some(f) = filter.name.as_ref() {
            match f {
                pco_pack::StringFilter::Equal(v) => {
                    where_clauses.push(format!("{} = ${}", "name", params.len() + 1));
                    params.push(v);
                }
                pco_pack::StringFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(format!("{} = ANY(${})", "name", params.len() + 1));
                        params.push(vals);
                    }
                }
            }
        }
        if let Some(f) = filter.time.as_ref() {
            match f {
                pco_pack::DateTimeFilter::Equal(v) => {
                    where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                    params.push(v);
                    where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                    params.push(v);
                }
                pco_pack::DateTimeFilter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(vals.first().unwrap());
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(vals.last().unwrap());
                    }
                }
                pco_pack::DateTimeFilter::Range { start, end } => {
                    where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                    params.push(start);
                    where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                    params.push(end);
                }
            }
        }
        let sql_where = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let filter_json: serde_json::Value = filter.clone().try_into()?;
        let fields = <Serde as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &[
            "id",
            "name",
            "start_at",
            "end_at",
            "time",
            "description",
            "tags",
            "nums",
            "map",
            "json",
            "model",
        ];
        let select_columns: Vec<_> = known
            .iter()
            .filter(|c| fields.contains(*c))
            .copied()
            .collect();
        let query_sql = format!(
            "DELETE FROM {} {} RETURNING {}", "serdes", sql_where, select_columns
            .join(", ")
        );
        let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
        let mut chunks: Vec<Self> = Vec::new();
        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
            let mut index = 0;
            chunks
                .push(Self {
                    chunk: Chunk {
                        id: if fields.contains(&"id") {
                            let v = row.get(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        name: if fields.contains(&"name") {
                            let v = row.get(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        start_at: if fields.contains(&"start_at") {
                            let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        end_at: if fields.contains(&"end_at") {
                            let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                            index += 1;
                            v
                        } else {
                            Default::default()
                        },
                        time: if fields.contains(&"time") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        description: if fields.contains(&"description") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        tags: if fields.contains(&"tags") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        nums: if fields.contains(&"nums") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        map: if fields.contains(&"map") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        json: if fields.contains(&"json") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        model: if fields.contains(&"model") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                    },
                    filter: filter.clone(),
                    fields: fields_vec.clone(),
                });
        }
        Ok(chunks)
    }
    /// Writes the data to disk.
    pub async fn store(
        db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        rows: Vec<Serde>,
    ) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let chunks = Serde::write(rows)?;
        let stmt = db
            .prepare_cached(
                &format!(
                    "COPY {} ({}) FROM STDIN BINARY", "serdes",
                    "id, name, start_at, end_at, time, description, tags, nums, map, json, model"
                ),
            )
            .await?;
        let types = &[
            tokio_postgres::types::Type::UUID,
            tokio_postgres::types::Type::TEXT,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
        ];
        let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(
            db.copy_in(&stmt).await?,
            types,
        );
        futures::pin_mut!(writer);
        for chunk in chunks {
            writer
                .as_mut()
                .write(
                    &[
                        &chunk.id,
                        &chunk.name,
                        &chunk.start_at,
                        &chunk.end_at,
                        &(*chunk.time).to_vec(),
                        &(*chunk.description).to_vec(),
                        &(*chunk.tags).to_vec(),
                        &(*chunk.nums).to_vec(),
                        &(*chunk.map).to_vec(),
                        &(*chunk.json).to_vec(),
                        &(*chunk.model).to_vec(),
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
}
