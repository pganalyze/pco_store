pub struct QueryStat {
    pub database_id: i64,
    pub calls: i64,
    pub total_time: f64,
}
pub struct QueryStats {
    pub database_id: i64,
    calls: Vec<u8>,
    total_time: Vec<u8>,
}
impl QueryStats {
    pub async fn load(
        db: &deadpool_postgres::Object,
        database_id: &[i64],
    ) -> anyhow::Result<Vec<QueryStats>> {
        if database_id.is_empty() {
            return ::anyhow::__private::Err({
                use ::anyhow::__private::kind::*;
                let error = match "database_id".to_string() + "must be specified" {
                    error => (&error).anyhow_kind().new(error),
                };
                error
            });
        }
        let sql = ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(
                format_args!(
                    "SELECT * FROM {0} WHERE {1}", "query_stats",
                    "database_id = ANY($1)",
                ),
            );
            res
        });
        let mut results = Vec::new();
        for row in db.query(&db.prepare_cached(&sql).await?, &[&database_id]).await? {
            results
                .push(QueryStats {
                    database_id: row.get(0usize),
                    calls: row.get(1usize),
                    total_time: row.get(2usize),
                });
        }
        Ok(results)
    }
    pub async fn delete(
        db: &deadpool_postgres::Object,
        database_id: &[i64],
    ) -> anyhow::Result<Vec<QueryStats>> {
        if database_id.is_empty() {
            return ::anyhow::__private::Err({
                use ::anyhow::__private::kind::*;
                let error = match "database_id".to_string() + "must be specified" {
                    error => (&error).anyhow_kind().new(error),
                };
                error
            });
        }
        let sql = ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(
                format_args!(
                    "DELETE FROM {0} WHERE {1} RETURNING *", "query_stats",
                    "database_id = ANY($1)",
                ),
            );
            res
        });
        let mut results = Vec::new();
        for row in db.query(&db.prepare_cached(&sql).await?, &[&database_id]).await? {
            results
                .push(QueryStats {
                    database_id: row.get(0usize),
                    calls: row.get(1usize),
                    total_time: row.get(2usize),
                });
        }
        Ok(results)
    }
    pub fn decompress(self) -> anyhow::Result<Vec<QueryStat>> {
        let mut results = Vec::new();
        let calls: Vec<i64> = if self.calls.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.calls)?
        };
        let total_time: Vec<i64> = if self.total_time.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.total_time)?
        };
        let len = [calls.len(), total_time.len()].into_iter().max().unwrap_or(0);
        for index in 0..len {
            let row = QueryStat {
                database_id: self.database_id,
                calls: calls.get(index).cloned().unwrap_or_default(),
                total_time: total_time.get(index).cloned().unwrap_or_default() as f64
                    / 100f32 as f64,
            };
            if true {
                results.push(row);
            }
        }
        Ok(results)
    }
    pub async fn store(
        db: &deadpool_postgres::Object,
        rows: Vec<QueryStat>,
    ) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut grouped_rows: std::collections::HashMap<_, Vec<QueryStat>> = std::collections::HashMap::new();
        for row in rows {
            grouped_rows.entry((row.database_id,)).or_default().push(row);
        }
        let sql = ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(
                format_args!(
                    "COPY {0} ({1}) FROM STDIN BINARY", "query_stats",
                    "database_id, calls, total_time",
                ),
            );
            res
        });
        let types = &[
            tokio_postgres::types::Type::INT8,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
        ];
        let stmt = db.copy_in(&db.prepare_cached(&sql).await?).await?;
        let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(stmt, types);
        let mut writer = writer;
        #[allow(unused_mut)]
        let mut writer = unsafe {
            ::pin_utils::core_reexport::pin::Pin::new_unchecked(&mut writer)
        };
        for rows in grouped_rows.into_values() {
            writer
                .as_mut()
                .write(
                    &[
                        &rows[0].database_id,
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.calls).collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows
                                    .iter()
                                    .map(|r| (r.total_time * 100f32 as f64).round() as i64)
                                    .collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
}
