use pco_pack::PcoPack;
#[derive(PcoPack)]
#[pco_pack(timestamp = collected_at, index = [database_id])]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub collected_secs: i64,
    pub fingerprint: i64,
    pub postgres_role_id: i64,
    pub calls: i64,
    pub rows: i64,
    pub total_time: f64,
    pub io_time: f64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
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
        if let Some(f) = filter.database_id.as_ref() {
            match f {
                pco_pack::I64Filter::Equal(v) => {
                    where_clauses
                        .push(format!("{} = ${}", "database_id", params.len() + 1));
                    params.push(v);
                }
                pco_pack::I64Filter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(
                                format!("{} = ANY(${})", "database_id", params.len() + 1),
                            );
                        params.push(vals);
                    }
                }
                pco_pack::I64Filter::Range { start, end } => {
                    where_clauses
                        .push(
                            format!(
                                "{} >= ${} AND {} <= ${}", "database_id", params.len() + 1,
                                "database_id", params.len() + 2
                            ),
                        );
                    params.push(start);
                    params.push(end);
                }
            }
        }
        if let Some(f) = filter.collected_at.as_ref() {
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
        let fields = <QueryStat as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &[
            "database_id",
            "start_at",
            "end_at",
            "collected_at",
            "collected_secs",
            "fingerprint",
            "postgres_role_id",
            "calls",
            "rows",
            "total_time",
            "io_time",
            "shared_blks_hit",
            "shared_blks_read",
        ];
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
                        collected_at: if fields.contains(&"collected_at") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        collected_secs: if fields.contains(&"collected_secs") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        fingerprint: if fields.contains(&"fingerprint") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        postgres_role_id: if fields.contains(&"postgres_role_id") {
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
                        rows: if fields.contains(&"rows") {
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
                        io_time: if fields.contains(&"io_time") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        shared_blks_hit: if fields.contains(&"shared_blks_hit") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        shared_blks_read: if fields.contains(&"shared_blks_read") {
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
        if let Some(f) = filter.database_id.as_ref() {
            match f {
                pco_pack::I64Filter::Equal(v) => {
                    where_clauses
                        .push(format!("{} = ${}", "database_id", params.len() + 1));
                    params.push(v);
                }
                pco_pack::I64Filter::Inclusion(vals) => {
                    if !vals.is_empty() {
                        where_clauses
                            .push(
                                format!("{} = ANY(${})", "database_id", params.len() + 1),
                            );
                        params.push(vals);
                    }
                }
                pco_pack::I64Filter::Range { start, end } => {
                    where_clauses
                        .push(
                            format!(
                                "{} >= ${} AND {} <= ${}", "database_id", params.len() + 1,
                                "database_id", params.len() + 2
                            ),
                        );
                    params.push(start);
                    params.push(end);
                }
            }
        }
        if let Some(f) = filter.collected_at.as_ref() {
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
        let fields = <QueryStat as PcoPack>::resolve_fields(&filter_json, fields)?;
        let known: &[&str] = &[
            "database_id",
            "start_at",
            "end_at",
            "collected_at",
            "collected_secs",
            "fingerprint",
            "postgres_role_id",
            "calls",
            "rows",
            "total_time",
            "io_time",
            "shared_blks_hit",
            "shared_blks_read",
        ];
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
                        collected_at: if fields.contains(&"collected_at") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        collected_secs: if fields.contains(&"collected_secs") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        fingerprint: if fields.contains(&"fingerprint") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        postgres_role_id: if fields.contains(&"postgres_role_id") {
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
                        rows: if fields.contains(&"rows") {
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
                        io_time: if fields.contains(&"io_time") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        shared_blks_hit: if fields.contains(&"shared_blks_hit") {
                            let v = row.get::<_, Option<&[u8]>>(index);
                            index += 1;
                            v.map_or_else(
                                Default::default,
                                pco_pack::serde_bytes::ByteBuf::from,
                            )
                        } else {
                            Default::default()
                        },
                        shared_blks_read: if fields.contains(&"shared_blks_read") {
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
                    "database_id, start_at, end_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read"
                ),
            )
            .await?;
        let types = &[
            tokio_postgres::types::Type::INT8,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
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
                        &chunk.database_id,
                        &chunk.start_at,
                        &chunk.end_at,
                        &(*chunk.collected_at).to_vec(),
                        &(*chunk.collected_secs).to_vec(),
                        &(*chunk.fingerprint).to_vec(),
                        &(*chunk.postgres_role_id).to_vec(),
                        &(*chunk.calls).to_vec(),
                        &(*chunk.rows).to_vec(),
                        &(*chunk.total_time).to_vec(),
                        &(*chunk.io_time).to_vec(),
                        &(*chunk.shared_blks_hit).to_vec(),
                        &(*chunk.shared_blks_read).to_vec(),
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
}
