use pco_pack::PcoPack;
#[derive(PcoPack)]
#[pco_pack()]
pub struct QueryStat {
    pub database_id: i64,
    pub calls: i64,
    pub total_time: f64,
}
/// Type alias for the compressed chunk representation.
pub type Chunk = <QueryStat as PcoPack>::Chunk;
/// Typed filter struct provided by pco_pack.
pub type Filter = <QueryStat as PcoPack>::Filter;
/// A single row of compressed data that can be decompressed on demand.
pub struct CompressedQueryStats {
    chunk: Chunk,
    filter: Filter,
    fields: Vec<String>,
}
impl std::ops::Deref for CompressedQueryStats {
    type Target = Chunk;
    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}
impl CompressedQueryStats {
    /// Decompresses this chunk using the filter and fields used to load it.
    pub fn decompress(&self) -> anyhow::Result<Vec<QueryStat>> {
        let fields: Vec<&str> = self.fields.iter().map(|s| s.as_str()).collect();
        <QueryStat as PcoPack>::filter(
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
        let sql_where = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let filter_json: serde_json::Value = filter.clone().try_into()?;
        let fields = <QueryStat as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &["database_id", "calls", "total_time"];
        let select_columns: Vec<_> = known
            .iter()
            .filter(|c| fields.contains(*c))
            .copied()
            .collect();
        let query_sql = format!(
            "SELECT {} FROM {} {}", select_columns.join(", "), "query_stats", sql_where
        );
        let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
        let mut chunks: Vec<Self> = Vec::new();
        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
            let mut index = 0;
            chunks
                .push(Self {
                    chunk: Chunk {
                        database_id: if fields.contains(&"database_id") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        calls: if fields.contains(&"calls") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        total_time: if fields.contains(&"total_time") {
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
        let sql_where = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let filter_json: serde_json::Value = filter.clone().try_into()?;
        let fields = <QueryStat as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &["database_id", "calls", "total_time"];
        let select_columns: Vec<_> = known
            .iter()
            .filter(|c| fields.contains(*c))
            .copied()
            .collect();
        let query_sql = format!(
            "DELETE FROM {} {} RETURNING {}", "query_stats", sql_where, select_columns
            .join(", ")
        );
        let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
        let mut chunks: Vec<Self> = Vec::new();
        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
            let mut index = 0;
            chunks
                .push(Self {
                    chunk: Chunk {
                        database_id: if fields.contains(&"database_id") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        calls: if fields.contains(&"calls") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        total_time: if fields.contains(&"total_time") {
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
        rows: Vec<QueryStat>,
    ) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let chunks = QueryStat::write(rows)?;
        let stmt = db
            .prepare_cached(
                &format!(
                    "COPY {} ({}) FROM STDIN BINARY", "query_stats",
                    "database_id, calls, total_time"
                ),
            )
            .await?;
        let types = &[
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
                        &(*chunk.database_id).to_vec(),
                        &(*chunk.calls).to_vec(),
                        &(*chunk.total_time).to_vec(),
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
}
