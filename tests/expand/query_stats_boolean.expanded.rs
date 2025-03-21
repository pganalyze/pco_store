pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: SystemTime,
    pub collected_secs: i64,
    pub fingerprint: i64,
    pub postgres_role_id: i64,
    pub toplevel: bool,
    pub calls: i64,
    pub rows: i64,
    pub total_time: f64,
    pub io_time: f64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
}
pub struct QueryStats {
    pub database_id: i64,
    pub filter_start: SystemTime,
    pub filter_end: SystemTime,
    pub start_at: SystemTime,
    pub end_at: SystemTime,
    collected_at: Vec<u8>,
    collected_secs: Vec<u8>,
    fingerprint: Vec<u8>,
    postgres_role_id: Vec<u8>,
    toplevel: Vec<u8>,
    calls: Vec<u8>,
    rows: Vec<u8>,
    total_time: Vec<u8>,
    io_time: Vec<u8>,
    shared_blks_hit: Vec<u8>,
    shared_blks_read: Vec<u8>,
}
impl QueryStats {
    pub async fn load(
        db: &deadpool_postgres::Object,
        database_id: &[i64],
        filter_start: SystemTime,
        filter_end: SystemTime,
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
                    "database_id = ANY($1) AND end_at >= $2 AND start_at <= $3",
                ),
            );
            res
        });
        let mut results = Vec::new();
        for row in db
            .query(
                &db.prepare_cached(&sql).await?,
                &[&database_id, &filter_start, &filter_end],
            )
            .await?
        {
            results
                .push(QueryStats {
                    database_id: row.get(0usize),
                    start_at: row.get(1usize),
                    end_at: row.get(2usize),
                    collected_at: row.get(3usize),
                    collected_secs: row.get(4usize),
                    fingerprint: row.get(5usize),
                    postgres_role_id: row.get(6usize),
                    toplevel: row.get(7usize),
                    calls: row.get(8usize),
                    rows: row.get(9usize),
                    total_time: row.get(10usize),
                    io_time: row.get(11usize),
                    shared_blks_hit: row.get(12usize),
                    shared_blks_read: row.get(13usize),
                    filter_start,
                    filter_end,
                });
        }
        Ok(results)
    }
    pub async fn delete(
        db: &deadpool_postgres::Object,
        database_id: &[i64],
        filter_start: SystemTime,
        filter_end: SystemTime,
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
                    "database_id = ANY($1) AND end_at >= $2 AND start_at <= $3",
                ),
            );
            res
        });
        let mut results = Vec::new();
        for row in db
            .query(
                &db.prepare_cached(&sql).await?,
                &[&database_id, &filter_start, &filter_end],
            )
            .await?
        {
            results
                .push(QueryStats {
                    database_id: row.get(0usize),
                    start_at: row.get(1usize),
                    end_at: row.get(2usize),
                    collected_at: row.get(3usize),
                    collected_secs: row.get(4usize),
                    fingerprint: row.get(5usize),
                    postgres_role_id: row.get(6usize),
                    toplevel: row.get(7usize),
                    calls: row.get(8usize),
                    rows: row.get(9usize),
                    total_time: row.get(10usize),
                    io_time: row.get(11usize),
                    shared_blks_hit: row.get(12usize),
                    shared_blks_read: row.get(13usize),
                    filter_start,
                    filter_end,
                });
        }
        Ok(results)
    }
    pub fn decompress(self) -> anyhow::Result<Vec<QueryStat>> {
        let mut results = Vec::new();
        let collected_at: Vec<u64> = if self.collected_at.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.collected_at)?
        };
        let collected_secs: Vec<i64> = if self.collected_secs.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.collected_secs)?
        };
        let fingerprint: Vec<i64> = if self.fingerprint.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.fingerprint)?
        };
        let postgres_role_id: Vec<i64> = if self.postgres_role_id.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.postgres_role_id)?
        };
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
        let rows: Vec<i64> = if self.rows.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.rows)?
        };
        let total_time: Vec<i64> = if self.total_time.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.total_time)?
        };
        let io_time: Vec<i64> = if self.io_time.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.io_time)?
        };
        let shared_blks_hit: Vec<i64> = if self.shared_blks_hit.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.shared_blks_hit)?
        };
        let shared_blks_read: Vec<i64> = if self.shared_blks_read.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.shared_blks_read)?
        };
        let len = [
            collected_at.len(),
            collected_secs.len(),
            fingerprint.len(),
            postgres_role_id.len(),
            toplevel.len(),
            calls.len(),
            rows.len(),
            total_time.len(),
            io_time.len(),
            shared_blks_hit.len(),
            shared_blks_read.len(),
        ]
            .into_iter()
            .max()
            .unwrap_or(0);
        for index in 0..len {
            let row = QueryStat {
                database_id: self.database_id,
                collected_at: SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_micros(collected_at[index]),
                collected_secs: collected_secs.get(index).cloned().unwrap_or_default(),
                fingerprint: fingerprint.get(index).cloned().unwrap_or_default(),
                postgres_role_id: postgres_role_id
                    .get(index)
                    .cloned()
                    .unwrap_or_default(),
                toplevel: toplevel.get(index).cloned().unwrap_or_default() == 1,
                calls: calls.get(index).cloned().unwrap_or_default(),
                rows: rows.get(index).cloned().unwrap_or_default(),
                total_time: total_time.get(index).cloned().unwrap_or_default() as f64
                    / 100f32 as f64,
                io_time: io_time.get(index).cloned().unwrap_or_default() as f64
                    / 100f32 as f64,
                shared_blks_hit: shared_blks_hit.get(index).cloned().unwrap_or_default(),
                shared_blks_read: shared_blks_read
                    .get(index)
                    .cloned()
                    .unwrap_or_default(),
            };
            if row.collected_at >= self.filter_start
                && row.collected_at <= self.filter_end
            {
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
                    "database_id, start_at, end_at, collected_at, collected_secs, fingerprint, postgres_role_id, toplevel, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read",
                ),
            );
            res
        });
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
            let collected_at: Vec<_> = rows.iter().map(|s| s.collected_at).collect();
            let start_at = *collected_at.iter().min().unwrap();
            let end_at = *collected_at.iter().max().unwrap();
            let collected_at: Vec<u64> = collected_at
                .into_iter()
                .map(|t| {
                    t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64
                })
                .collect();
            writer
                .as_mut()
                .write(
                    &[
                        &rows[0].database_id,
                        &start_at,
                        &end_at,
                        &::pco::standalone::simpler_compress(
                                &collected_at,
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.collected_secs).collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.fingerprint).collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows
                                    .iter()
                                    .map(|r| r.postgres_role_id)
                                    .collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
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
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.rows).collect::<Vec<_>>(),
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
                        &::pco::standalone::simpler_compress(
                                &rows
                                    .iter()
                                    .map(|r| (r.io_time * 100f32 as f64).round() as i64)
                                    .collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows.iter().map(|r| r.shared_blks_hit).collect::<Vec<_>>(),
                                ::pco::DEFAULT_COMPRESSION_LEVEL,
                            )
                            .unwrap(),
                        &::pco::standalone::simpler_compress(
                                &rows
                                    .iter()
                                    .map(|r| r.shared_blks_read)
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
