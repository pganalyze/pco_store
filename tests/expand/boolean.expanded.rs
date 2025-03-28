pub struct QueryStat {
    pub database_id: i64,
    pub toplevel: bool,
    pub calls: i64,
}
pub struct QueryStats {
    pub database_id: i64,
    toplevel: Vec<u8>,
    calls: Vec<u8>,
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
                    toplevel: row.get(1usize),
                    calls: row.get(2usize),
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
                    toplevel: row.get(1usize),
                    calls: row.get(2usize),
                });
        }
        Ok(results)
    }
    pub fn decompress(self) -> anyhow::Result<Vec<QueryStat>> {
        let mut results = Vec::new();
        let toplevel: Vec<u16> = if self.toplevel.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.toplevel)?
        };
        let calls: Vec<i64> = if self.calls.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.calls)?
        };
        let len = [toplevel.len(), calls.len()].into_iter().max().unwrap_or(0);
        for index in 0..len {
            let row = QueryStat {
                database_id: self.database_id,
                toplevel: toplevel.get(index).cloned().unwrap_or_default() == 1,
                calls: calls.get(index).cloned().unwrap_or_default(),
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
                    "database_id, toplevel, calls",
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
                                &rows.iter().map(|r| r.toplevel as u16).collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.calls).collect::<Vec<_>>(),
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
