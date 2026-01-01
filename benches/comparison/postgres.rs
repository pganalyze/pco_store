use super::*;

pub async fn store() -> Result<()> {
    let db = &mut DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS comparison_postgres;
        CREATE TABLE comparison_postgres (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at timestamptz[] NOT NULL,
            collected_secs bigint[] NOT NULL,
            fingerprint bigint[] NOT NULL,
            postgres_role_id bigint[] NOT NULL,
            calls bigint[] NOT NULL,
            rows bigint[] NOT NULL,
            total_time bigint[] NOT NULL,
            io_time bigint[] NOT NULL,
            shared_blks_hit bigint[] NOT NULL,
            shared_blks_read bigint[] NOT NULL
        );
        CREATE INDEX ON comparison_postgres USING btree (database_id);
        CREATE INDEX ON comparison_postgres USING btree (end_at, start_at);
    ";
    db.batch_execute(sql).await?;
    let database_ids: Vec<i64> = db.query_one("SELECT array_agg(DISTINCT database_id) FROM query_stats", &[]).await?.get(0);
    for database_id in database_ids {
        let sql = "
            SELECT start_at, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
            FROM query_stats WHERE database_id = $1
        ";
        let rows = db.query(sql, &[&database_id]).await?;
        let mut grouped_rows: ahash::AHashMap<_, Vec<_>> = ahash::AHashMap::new();
        for row in rows {
            let time: DateTime<Utc> = row.get::<_, SystemTime>(0).into();
            let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
            grouped_rows.entry(date).or_default().push(row);
        }
        let sql = "
            COPY comparison_postgres (
                database_id, start_at, end_at,
                collected_at, collected_secs, fingerprint, postgres_role_id,
                calls, rows, total_time, io_time,
                shared_blks_hit, shared_blks_read
            ) FROM STDIN BINARY
        ";
        #[rustfmt::skip]
        let types = &[
            Type::INT8, Type::TIMESTAMPTZ, Type::TIMESTAMPTZ,
            Type::TIMESTAMPTZ_ARRAY, Type::INT8_ARRAY, Type::INT8_ARRAY, Type::INT8_ARRAY,
            Type::INT8_ARRAY, Type::INT8_ARRAY, Type::INT8_ARRAY, Type::INT8_ARRAY,
            Type::INT8_ARRAY, Type::INT8_ARRAY
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
            let mut total_time: Vec<i64> = Vec::new();
            let mut io_time: Vec<i64> = Vec::new();
            let mut shared_blks_hit: Vec<i64> = Vec::new();
            let mut shared_blks_read: Vec<i64> = Vec::new();
            for row in rows_ {
                collected_at.append(&mut row.get(1));
                collected_secs.append(&mut row.get(2));
                fingerprint.append(&mut row.get(3));
                postgres_role_id.append(&mut row.get(4));
                calls.append(&mut row.get(5));
                rows.append(&mut row.get(6));
                total_time.append(&mut row.get::<_, Vec<f64>>(7).into_iter().map(|t| (t * 100.0).round() as i64).collect());
                io_time.append(&mut row.get::<_, Vec<f64>>(8).into_iter().map(|t| (t * 100.0).round() as i64).collect());
                shared_blks_hit.append(&mut row.get(9));
                shared_blks_read.append(&mut row.get(10));
            }
            let start_at = *collected_at.iter().min().unwrap();
            let end_at = *collected_at.iter().max().unwrap();
            #[rustfmt::skip]
            let values: &[&(dyn ToSql + Sync)] = &[
                &database_id, &start_at, &end_at,
                &collected_at, &collected_secs, &fingerprint, &postgres_role_id,
                &calls, &rows, &total_time, &io_time,
                &shared_blks_hit, &shared_blks_read,
            ];
            writer.as_mut().write(values).await?;
        }
        writer.finish().await?;
    }
    Ok(())
}

pub async fn load() -> Result<()> {
    let db = &DB_POOL.get().await.unwrap();
    let mut results = Vec::new();
    let sql = "
        SELECT database_id, collected_at, collected_secs, fingerprint, postgres_role_id, calls, rows, total_time, io_time, shared_blks_hit, shared_blks_read
        FROM comparison_postgres
    ";
    for row in db.query(sql, &[]).await? {
        let database_id: i64 = row.get(0);
        let collected_at: Vec<SystemTime> = row.get(1);
        let collected_secs: Vec<i64> = row.get(2);
        let fingerprint: Vec<i64> = row.get(3);
        let postgres_role_id: Vec<i64> = row.get(4);
        let calls: Vec<i64> = row.get(5);
        let rows: Vec<i64> = row.get(6);
        let total_time: Vec<i64> = row.get(7);
        let io_time: Vec<i64> = row.get(8);
        let shared_blks_hit: Vec<i64> = row.get(9);
        let shared_blks_read: Vec<i64> = row.get(10);
        for (index, collected_at) in collected_at.into_iter().enumerate() {
            results.push((
                database_id,
                collected_at,
                collected_secs[index],
                fingerprint[index],
                postgres_role_id[index],
                calls[index],
                rows[index],
                total_time[index] as f64 / 100.0,
                io_time[index] as f64 / 100.0,
                shared_blks_hit[index],
                shared_blks_read[index],
            ));
        }
    }
    Ok(())
}
