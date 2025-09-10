use super::*;

pub async fn store(decimals: u8) -> Result<()> {
    let db = &mut DB_POOL.get().await.unwrap();
    let sql = &format!(
        "
        DROP TABLE IF EXISTS float_round_{decimals};
        CREATE TABLE float_round_{decimals} (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL,
            collected_secs bytea STORAGE EXTERNAL NOT NULL,
            fingerprint bytea STORAGE EXTERNAL NOT NULL,
            postgres_role_id bytea STORAGE EXTERNAL NOT NULL,
            calls bytea STORAGE EXTERNAL NOT NULL,
            rows bytea STORAGE EXTERNAL NOT NULL,
            total_time bytea STORAGE EXTERNAL NOT NULL,
            io_time bytea STORAGE EXTERNAL NOT NULL,
            shared_blks_hit bytea STORAGE EXTERNAL NOT NULL,
            shared_blks_read bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON float_round_{decimals} USING btree (database_id);
        CREATE INDEX ON float_round_{decimals} USING btree (end_at, start_at);
    "
    );
    db.batch_execute(sql).await?;
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM query_stats", &[]).await?.get(0);
    for database_id in database_ids {
        let sql = "
            SELECT start_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
            FROM query_stats WHERE database_id = $1
        ";
        let rows = db.query(sql, &[&database_id]).await?;
        let mut grouped_rows: HashMap<_, Vec<_>> = HashMap::new();
        for row in rows {
            let time: DateTime<Utc> = row.get::<_, SystemTime>(0).into();
            let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
            grouped_rows.entry(date).or_default().push(row);
        }
        let sql = &format!(
            "
            COPY float_round_{decimals} (
                database_id, start_at, end_at, collected_at, collected_secs,
                fingerprint, postgres_role_id, calls, rows, total_time,
                io_time, shared_blks_hit, shared_blks_read
            ) FROM STDIN BINARY
        "
        );
        #[rustfmt::skip]
        let types: &'static [Type] = &[
            Type::INT8, Type::TIMESTAMPTZ, Type::TIMESTAMPTZ, Type::BYTEA, Type::BYTEA,
            Type::BYTEA, Type::BYTEA, Type::BYTEA, Type::BYTEA, Type::BYTEA,
            Type::BYTEA, Type::BYTEA, Type::BYTEA,
        ];
        let writer = BinaryCopyInWriter::new(db.copy_in(&db.prepare_cached(sql).await?).await?, types);
        futures::pin_mut!(writer);
        for rows_ in grouped_rows.into_values() {
            let mut collected_at: Vec<SystemTime> = Vec::new();
            let mut collected_secs: Vec<i64> = Vec::new();
            let mut fingerprint: Vec<i64> = Vec::new();
            let mut postgres_role_id: Vec<i64> = Vec::new();
            let mut calls: Vec<i64> = Vec::new();
            let mut rows: Vec<i64> = Vec::new();
            let mut total_time: Vec<f64> = Vec::new();
            let mut io_time: Vec<f64> = Vec::new();
            let mut shared_blks_hit: Vec<i64> = Vec::new();
            let mut shared_blks_read: Vec<i64> = Vec::new();
            for row in rows_ {
                collected_at.append(&mut row.get(1));
                collected_secs.append(&mut row.get(2));
                fingerprint.append(&mut row.get(3));
                postgres_role_id.append(&mut row.get(4));
                calls.append(&mut row.get(5));
                rows.append(&mut row.get(6));
                total_time.append(&mut row.get(7));
                io_time.append(&mut row.get(8));
                shared_blks_hit.append(&mut row.get(9));
                shared_blks_read.append(&mut row.get(10));
            }
            total_time = total_time.into_iter().map(|t| round(t, decimals)).collect();
            io_time = io_time.into_iter().map(|t| round(t, decimals)).collect();
            let start_at = *collected_at.iter().min().unwrap();
            let end_at = *collected_at.iter().max().unwrap();
            let collected_at: Vec<u64> =
                collected_at.into_iter().map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64).collect();
            #[rustfmt::skip]
            let values: &[&(dyn ToSql + Sync)] = &[
                &database_id, &start_at, &end_at,
                &pco::standalone::simpler_compress(&collected_at, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&collected_secs, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&fingerprint, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&postgres_role_id, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&calls, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&rows, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&total_time, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&io_time, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&shared_blks_hit, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
                &pco::standalone::simpler_compress(&shared_blks_read, pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
            ];
            writer.as_mut().write(values).await?;
        }
        writer.finish().await?;
    }
    Ok(())
}

fn round(float: f64, decimals: u8) -> f64 {
    let shift = decimals as i32 + 1 - float.abs().log10().ceil() as i32;
    let shift_factor = 10f64.powi(shift);
    (float * shift_factor).round() / shift_factor
}

pub async fn load(decimals: u8) -> Result<()> {
    let db = &DB_POOL.get().await.unwrap();
    let mut stats = Vec::new();
    for group in CompressedQueryStats::load(db, decimals).await? {
        for stat in group.decompress() {
            stats.push(stat);
        }
    }
    return Ok(());

    #[derive(Debug)]
    pub struct QueryStat {
        pub database_id: i64,
        pub collected_at: SystemTime,
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
    pub struct CompressedQueryStats {
        database_id: i64,
        start_at: SystemTime,
        collected_at: Vec<u8>,
        collected_secs: Vec<u8>,
        fingerprint: Vec<u8>,
        postgres_role_id: Vec<u8>,
        calls: Vec<u8>,
        rows: Vec<u8>,
        total_time: Vec<u8>,
        io_time: Vec<u8>,
        shared_blks_hit: Vec<u8>,
        shared_blks_read: Vec<u8>,
    }
    impl CompressedQueryStats {
        pub async fn load(db: &Client, decimals: u8) -> Result<Vec<Self>> {
            let sql = &format!("
                SELECT database_id, start_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
                FROM float_round_{decimals}
            ");
            let mut results = Vec::new();
            for row in db.query(&db.prepare_cached(sql).await?, &[]).await? {
                results.push(Self {
                    database_id: row.get(0),
                    start_at: row.get(1),
                    collected_at: row.get(2),
                    collected_secs: row.get(3),
                    fingerprint: row.get(4),
                    postgres_role_id: row.get(5),
                    calls: row.get(6),
                    rows: row.get(7),
                    total_time: row.get(8),
                    io_time: row.get(9),
                    shared_blks_hit: row.get(10),
                    shared_blks_read: row.get(11),
                });
            }
            Ok(results)
        }

        pub fn decompress(self) -> Vec<QueryStat> {
            let mut results = Vec::new();
            let collected_at: Vec<u64> = pco::standalone::simple_decompress(&self.collected_at).unwrap();
            let collected_secs: Vec<i64> = pco::standalone::simple_decompress(&self.collected_secs).unwrap();
            let fingerprint: Vec<i64> = pco::standalone::simple_decompress(&self.fingerprint).unwrap();
            let postgres_role_id: Vec<i64> = pco::standalone::simple_decompress(&self.postgres_role_id).unwrap();
            let calls: Vec<i64> = pco::standalone::simple_decompress(&self.calls).unwrap();
            let rows: Vec<i64> = pco::standalone::simple_decompress(&self.rows).unwrap();
            let total_time: Vec<f64> = pco::standalone::simple_decompress(&self.total_time).unwrap();
            let io_time: Vec<f64> = pco::standalone::simple_decompress(&self.io_time).unwrap();
            let shared_blks_hit: Vec<i64> = pco::standalone::simple_decompress(&self.shared_blks_hit).unwrap();
            let shared_blks_read: Vec<i64> = pco::standalone::simple_decompress(&self.shared_blks_read).unwrap();
            for (index, collected_at) in collected_at.into_iter().enumerate() {
                let collected_at = SystemTime::UNIX_EPOCH + Duration::from_micros(collected_at);
                results.push(QueryStat {
                    database_id: self.database_id,
                    collected_at,
                    collected_secs: collected_secs[index],
                    fingerprint: fingerprint[index],
                    postgres_role_id: postgres_role_id[index],
                    calls: calls[index],
                    rows: rows[index],
                    total_time: total_time[index],
                    io_time: io_time[index],
                    shared_blks_hit: shared_blks_hit[index],
                    shared_blks_read: shared_blks_read[index],
                });
            }
            results
        }
    }
}
