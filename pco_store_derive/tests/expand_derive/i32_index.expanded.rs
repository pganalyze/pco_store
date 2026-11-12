#[derive(pco_store::pco_pack::PcoPack)]
pub struct QueryStat {
    pub database_id: i64,
    pub granularity: i32,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub calls: i64,
}
impl pco_store::PcoStore for QueryStat {}
impl pco_store::Backend<QueryStat> for pco_store::fs::Fs {
    fn store<'a>(
        &'a self,
        chunks: &'a [<QueryStat as pco_store::pco_pack::PcoPack>::Chunk],
    ) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<()>> {
        Box::pin(async move {
            for chunk in chunks {
                let bytes = pco_store::encode_chunk::<QueryStat>(chunk)?;
                let mut key_parts: Vec<String> = Vec::new();
                key_parts.push(pco_store::fs::sanitize(&chunk.database_id.to_string()));
                key_parts.push(pco_store::fs::sanitize(&chunk.granularity.to_string()));
                let bounds = format!(
                    "{}.{}", chunk.start_at.timestamp_micros(), chunk.end_at
                    .timestamp_micros()
                );
                let name = format!(
                    "{}~{}~{}~{}.bin", "query_stats", key_parts.join("|"), bounds,
                    pco_store::fs::fnv1a_hex(& bytes)
                );
                self.dur().put(&name, &bytes)?;
            }
            Ok(())
        })
    }
    fn load<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
        fields: &'a [&str],
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            let filter_json: pco_store::pco_pack::serde_json::Value = filter
                .clone()
                .try_into()?;
            let resolved = <QueryStat as pco_store::pco_pack::PcoPack>::resolve_fields(
                &filter_json,
                fields,
            )?;
            let resolved_fields: Vec<String> = resolved
                .iter()
                .map(|f| (*f).to_string())
                .collect();
            let narrows = |
                name: &str,
                filter: &<QueryStat as pco_store::pco_pack::PcoPack>::Filter,
            | -> bool {
                let Some(rest) = name.strip_prefix(concat!("query_stats", "~")) else {
                    return false
                };
                let mut segments = rest.splitn(3, '~');
                let (Some(keys), Some(bounds), Some(digest)) = (
                    segments.next(),
                    segments.next(),
                    segments.next(),
                ) else { return false };
                if digest.is_empty() {
                    return false;
                }
                let keys: Vec<&str> = if keys.is_empty() {
                    Vec::new()
                } else {
                    keys.split('|').collect()
                };
                if let (Some(f), Some(k)) = (
                    filter.database_id.as_ref(),
                    keys.get(0usize).copied(),
                ) {
                    match f {
                        pco_store::pco_pack::I64Filter::Equal(v) => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed != *v {
                                    return false;
                                }
                            }
                        }
                        pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                            if !vals
                                .iter()
                                .any(|v| k.parse::<i64>().map(|p| p == *v).unwrap_or(true))
                            {
                                return false;
                            }
                        }
                        pco_store::pco_pack::I64Filter::Range { start, end } => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed < *start || parsed > *end {
                                    return false;
                                }
                            }
                        }
                    }
                }
                if let (Some(f), Some(k)) = (
                    filter.granularity.as_ref(),
                    keys.get(1usize).copied(),
                ) {
                    match f {
                        pco_store::pco_pack::I64Filter::Equal(v) => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed != *v {
                                    return false;
                                }
                            }
                        }
                        pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                            if !vals
                                .iter()
                                .any(|v| k.parse::<i64>().map(|p| p == *v).unwrap_or(true))
                            {
                                return false;
                            }
                        }
                        pco_store::pco_pack::I64Filter::Range { start, end } => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed < *start || parsed > *end {
                                    return false;
                                }
                            }
                        }
                    }
                }
                if let Some(f) = filter.collected_at.as_ref() {
                    if let Some((head, tail)) = bounds.split_once('.') {
                        if let (Ok(chunk_start), Ok(chunk_end)) = (
                            head.parse::<i64>(),
                            tail.parse::<i64>(),
                        ) {
                            match f {
                                pco_store::pco_pack::TimeFilter::Equal(v) => {
                                    let at = v.timestamp_micros();
                                    if chunk_end < at || chunk_start > at {
                                        return false;
                                    }
                                }
                                pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                                    if let (Some(first), Some(last)) = (
                                        vals.first(),
                                        vals.last(),
                                    ) {
                                        if chunk_end < first.timestamp_micros()
                                            || chunk_start > last.timestamp_micros()
                                        {
                                            return false;
                                        }
                                    }
                                }
                                pco_store::pco_pack::TimeFilter::Range { start, end } => {
                                    if chunk_end < start.timestamp_micros()
                                        || chunk_start > end.timestamp_micros()
                                    {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                true
            };
            let mut chunks = Vec::new();
            for found in self.dur().list_names()? {
                let name = found.to_string_lossy().into_owned();
                if !narrows(&name, filter) {
                    continue;
                }
                let Some(bytes) = self.dur().get(&name)? else { continue };
                let chunk = pco_store::decode_chunk::<QueryStat>(&bytes)?;
                chunks
                    .push(pco_store::Chunk {
                        chunk,
                        filter: filter.clone(),
                        fields: resolved_fields.clone(),
                    });
            }
            Ok(chunks)
        })
    }
    fn delete<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            let narrows = |
                name: &str,
                filter: &<QueryStat as pco_store::pco_pack::PcoPack>::Filter,
            | -> bool {
                let Some(rest) = name.strip_prefix(concat!("query_stats", "~")) else {
                    return false
                };
                let mut segments = rest.splitn(3, '~');
                let (Some(keys), Some(bounds), Some(digest)) = (
                    segments.next(),
                    segments.next(),
                    segments.next(),
                ) else { return false };
                if digest.is_empty() {
                    return false;
                }
                let keys: Vec<&str> = if keys.is_empty() {
                    Vec::new()
                } else {
                    keys.split('|').collect()
                };
                if let (Some(f), Some(k)) = (
                    filter.database_id.as_ref(),
                    keys.get(0usize).copied(),
                ) {
                    match f {
                        pco_store::pco_pack::I64Filter::Equal(v) => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed != *v {
                                    return false;
                                }
                            }
                        }
                        pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                            if !vals
                                .iter()
                                .any(|v| k.parse::<i64>().map(|p| p == *v).unwrap_or(true))
                            {
                                return false;
                            }
                        }
                        pco_store::pco_pack::I64Filter::Range { start, end } => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed < *start || parsed > *end {
                                    return false;
                                }
                            }
                        }
                    }
                }
                if let (Some(f), Some(k)) = (
                    filter.granularity.as_ref(),
                    keys.get(1usize).copied(),
                ) {
                    match f {
                        pco_store::pco_pack::I64Filter::Equal(v) => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed != *v {
                                    return false;
                                }
                            }
                        }
                        pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                            if !vals
                                .iter()
                                .any(|v| k.parse::<i64>().map(|p| p == *v).unwrap_or(true))
                            {
                                return false;
                            }
                        }
                        pco_store::pco_pack::I64Filter::Range { start, end } => {
                            if let Ok(parsed) = k.parse::<i64>() {
                                if parsed < *start || parsed > *end {
                                    return false;
                                }
                            }
                        }
                    }
                }
                if let Some(f) = filter.collected_at.as_ref() {
                    if let Some((head, tail)) = bounds.split_once('.') {
                        if let (Ok(chunk_start), Ok(chunk_end)) = (
                            head.parse::<i64>(),
                            tail.parse::<i64>(),
                        ) {
                            match f {
                                pco_store::pco_pack::TimeFilter::Equal(v) => {
                                    let at = v.timestamp_micros();
                                    if chunk_end < at || chunk_start > at {
                                        return false;
                                    }
                                }
                                pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                                    if let (Some(first), Some(last)) = (
                                        vals.first(),
                                        vals.last(),
                                    ) {
                                        if chunk_end < first.timestamp_micros()
                                            || chunk_start > last.timestamp_micros()
                                        {
                                            return false;
                                        }
                                    }
                                }
                                pco_store::pco_pack::TimeFilter::Range { start, end } => {
                                    if chunk_end < start.timestamp_micros()
                                        || chunk_start > end.timestamp_micros()
                                    {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                true
            };
            let mut chunks = Vec::new();
            for found in self.dur().list_names()? {
                let name = found.to_string_lossy().into_owned();
                if !narrows(&name, filter) {
                    continue;
                }
                let Some(bytes) = self.dur().get(&name)? else { continue };
                let chunk = pco_store::decode_chunk::<QueryStat>(&bytes)?;
                self.dur().remove(&name)?;
                chunks
                    .push(pco_store::Chunk {
                        chunk,
                        filter: Default::default(),
                        fields: vec![
                            "database_id".to_string(), "granularity".to_string(),
                            "collected_at".to_string(), "calls".to_string()
                        ],
                    });
            }
            Ok(chunks)
        })
    }
    fn capabilities(&self) -> pco_store::Capabilities {
        pco_store::Capabilities {
            transactions: false,
            rename: true,
            range_get: false,
            sql_pushdown: false,
        }
    }
}
impl pco_store::Backend<QueryStat> for pco_store::pg::Postgres {
    fn store<'a>(
        &'a self,
        chunks: &'a [<QueryStat as pco_store::pco_pack::PcoPack>::Chunk],
    ) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<()>> {
        Box::pin(async move {
            if chunks.is_empty() {
                return Ok(());
            }
            let db = self.client().await?;
            let stmt = db
                .prepare_cached(
                    &format!(
                        "COPY {} ({}) FROM STDIN BINARY", "query_stats",
                        "database_id, granularity, start_at, end_at, collected_at, calls"
                    ),
                )
                .await?;
            let types: &[pco_store::tokio_postgres::types::Type] = &[
                pco_store::tokio_postgres::types::Type::INT8,
                pco_store::tokio_postgres::types::Type::INT4,
                pco_store::tokio_postgres::types::Type::TIMESTAMPTZ,
                pco_store::tokio_postgres::types::Type::TIMESTAMPTZ,
                pco_store::tokio_postgres::types::Type::BYTEA,
                pco_store::tokio_postgres::types::Type::BYTEA,
            ];
            let mut writer = std::pin::pin!(
                pco_store::tokio_postgres::binary_copy::BinaryCopyInWriter::new(db
                .copy_in(& stmt). await ?, types)
            );
            for chunk in chunks {
                writer
                    .as_mut()
                    .write(
                        &[
                            &chunk.database_id,
                            &chunk.granularity,
                            &chunk.start_at,
                            &chunk.end_at,
                            &chunk.collected_at.as_slice(),
                            &chunk.calls.as_slice(),
                        ],
                    )
                    .await?;
            }
            writer.finish().await?;
            Ok(())
        })
    }
    fn load<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
        fields: &'a [&str],
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            type ChunkT = <QueryStat as pco_store::pco_pack::PcoPack>::Chunk;
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
            if let Some(f) = filter.database_id.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(v) => {
                        where_clauses
                            .push(format!("{} = ${}", "database_id", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(format!("{} = ${}", "database_id", params.len() + 1));
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!("{} = ANY(${})", "database_id", params.len() + 1),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
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
            if let Some(f) = filter.granularity.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(val) => {
                        where_clauses
                            .push(
                                format!("{} = ${}::bigint", "granularity", params.len() + 1),
                            );
                        params.push(val);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(
                                        format!("{} = ${}::bigint", "granularity", params.len() + 1),
                                    );
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!(
                                            "{} = ANY(${}::bigint[])", "granularity", params.len() + 1
                                        ),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
                        where_clauses
                            .push(
                                format!(
                                    "{} >= ${}::bigint AND {} <= ${}::bigint", "granularity",
                                    params.len() + 1, "granularity", params.len() + 2
                                ),
                            );
                        params.push(start);
                        params.push(end);
                    }
                }
            }
            if let Some(f) = filter.collected_at.as_ref() {
                match f {
                    pco_store::pco_pack::TimeFilter::Equal(v) => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(v);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                        if !vals.is_empty() {
                            where_clauses
                                .push(format!("end_at >= ${}", params.len() + 1));
                            params.push(vals.first().unwrap());
                            where_clauses
                                .push(format!("start_at <= ${}", params.len() + 1));
                            params.push(vals.last().unwrap());
                        }
                    }
                    pco_store::pco_pack::TimeFilter::Range { start, end } => {
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
            let filter_json: pco_store::pco_pack::serde_json::Value = filter
                .clone()
                .try_into()?;
            let fields = <QueryStat as pco_store::pco_pack::PcoPack>::resolve_fields(
                &filter_json,
                fields,
            )?;
            let known: &[&str] = &[
                "database_id",
                "granularity",
                "start_at",
                "end_at",
                "collected_at",
                "calls",
            ];
            let select_columns: Vec<&str> = known
                .iter()
                .copied()
                .filter(|c| fields.iter().any(|f| f == c))
                .collect();
            let query_sql = format!(
                "SELECT {} FROM {} {}", select_columns.join(", "), "query_stats",
                sql_where
            );
            let resolved_fields: Vec<String> = fields
                .iter()
                .map(|f| (*f).to_string())
                .collect();
            let db = self.client().await?;
            let mut chunks = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks
                    .push(pco_store::Chunk {
                        chunk: ChunkT {
                            database_id: if fields.contains(&"database_id") {
                                let v = row.get::<_, i64>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            granularity: if fields.contains(&"granularity") {
                                let v = row.get::<_, i32>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            start_at: if fields.contains(&"start_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            end_at: if fields.contains(&"end_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            collected_at: if fields.contains(&"collected_at") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                            calls: if fields.contains(&"calls") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                        },
                        filter: filter.clone(),
                        fields: resolved_fields.clone(),
                    });
            }
            Ok(chunks)
        })
    }
    /// Deletes matching rows and returns whole chunks with empty
    /// filters, so decompress answers every row the delete touched and
    /// a re-store loses nothing outside the filter range.
    fn delete<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            type ChunkT = <QueryStat as pco_store::pco_pack::PcoPack>::Chunk;
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
            if let Some(f) = filter.database_id.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(v) => {
                        where_clauses
                            .push(format!("{} = ${}", "database_id", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(format!("{} = ${}", "database_id", params.len() + 1));
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!("{} = ANY(${})", "database_id", params.len() + 1),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
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
            if let Some(f) = filter.granularity.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(val) => {
                        where_clauses
                            .push(
                                format!("{} = ${}::bigint", "granularity", params.len() + 1),
                            );
                        params.push(val);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(
                                        format!("{} = ${}::bigint", "granularity", params.len() + 1),
                                    );
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!(
                                            "{} = ANY(${}::bigint[])", "granularity", params.len() + 1
                                        ),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
                        where_clauses
                            .push(
                                format!(
                                    "{} >= ${}::bigint AND {} <= ${}::bigint", "granularity",
                                    params.len() + 1, "granularity", params.len() + 2
                                ),
                            );
                        params.push(start);
                        params.push(end);
                    }
                }
            }
            if let Some(f) = filter.collected_at.as_ref() {
                match f {
                    pco_store::pco_pack::TimeFilter::Equal(v) => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(v);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                        if !vals.is_empty() {
                            where_clauses
                                .push(format!("end_at >= ${}", params.len() + 1));
                            params.push(vals.first().unwrap());
                            where_clauses
                                .push(format!("start_at <= ${}", params.len() + 1));
                            params.push(vals.last().unwrap());
                        }
                    }
                    pco_store::pco_pack::TimeFilter::Range { start, end } => {
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
            let filter_json: pco_store::pco_pack::serde_json::Value = filter
                .clone()
                .try_into()?;
            let fields = <QueryStat as pco_store::pco_pack::PcoPack>::resolve_fields(
                &filter_json,
                &[],
            )?;
            let known: &[&str] = &[
                "database_id",
                "granularity",
                "start_at",
                "end_at",
                "collected_at",
                "calls",
            ];
            let select_columns: Vec<&str> = known
                .iter()
                .copied()
                .filter(|c| fields.iter().any(|f| f == c))
                .collect();
            let query_sql = format!(
                "DELETE FROM {} {} RETURNING {}", "query_stats", sql_where,
                select_columns.join(", ")
            );
            let resolved_fields: Vec<String> = fields
                .iter()
                .map(|f| (*f).to_string())
                .collect();
            let db = self.client().await?;
            let mut chunks = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks
                    .push(pco_store::Chunk {
                        chunk: ChunkT {
                            database_id: if fields.contains(&"database_id") {
                                let v = row.get::<_, i64>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            granularity: if fields.contains(&"granularity") {
                                let v = row.get::<_, i32>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            start_at: if fields.contains(&"start_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            end_at: if fields.contains(&"end_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            collected_at: if fields.contains(&"collected_at") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                            calls: if fields.contains(&"calls") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                        },
                        filter: Default::default(),
                        fields: resolved_fields.clone(),
                    });
            }
            Ok(chunks)
        })
    }
    fn capabilities(&self) -> pco_store::Capabilities {
        pco_store::Capabilities {
            transactions: false,
            rename: false,
            range_get: false,
            sql_pushdown: true,
        }
    }
}
impl pco_store::Backend<QueryStat> for pco_store::deadpool_postgres::Object {
    fn store<'a>(
        &'a self,
        chunks: &'a [<QueryStat as pco_store::pco_pack::PcoPack>::Chunk],
    ) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<()>> {
        Box::pin(async move {
            if chunks.is_empty() {
                return Ok(());
            }
            let db = self;
            let stmt = db
                .prepare_cached(
                    &format!(
                        "COPY {} ({}) FROM STDIN BINARY", "query_stats",
                        "database_id, granularity, start_at, end_at, collected_at, calls"
                    ),
                )
                .await?;
            let types: &[pco_store::tokio_postgres::types::Type] = &[
                pco_store::tokio_postgres::types::Type::INT8,
                pco_store::tokio_postgres::types::Type::INT4,
                pco_store::tokio_postgres::types::Type::TIMESTAMPTZ,
                pco_store::tokio_postgres::types::Type::TIMESTAMPTZ,
                pco_store::tokio_postgres::types::Type::BYTEA,
                pco_store::tokio_postgres::types::Type::BYTEA,
            ];
            let mut writer = std::pin::pin!(
                pco_store::tokio_postgres::binary_copy::BinaryCopyInWriter::new(db
                .copy_in(& stmt). await ?, types)
            );
            for chunk in chunks {
                writer
                    .as_mut()
                    .write(
                        &[
                            &chunk.database_id,
                            &chunk.granularity,
                            &chunk.start_at,
                            &chunk.end_at,
                            &chunk.collected_at.as_slice(),
                            &chunk.calls.as_slice(),
                        ],
                    )
                    .await?;
            }
            writer.finish().await?;
            Ok(())
        })
    }
    fn load<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
        fields: &'a [&str],
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            type ChunkT = <QueryStat as pco_store::pco_pack::PcoPack>::Chunk;
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
            if let Some(f) = filter.database_id.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(v) => {
                        where_clauses
                            .push(format!("{} = ${}", "database_id", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(format!("{} = ${}", "database_id", params.len() + 1));
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!("{} = ANY(${})", "database_id", params.len() + 1),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
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
            if let Some(f) = filter.granularity.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(val) => {
                        where_clauses
                            .push(
                                format!("{} = ${}::bigint", "granularity", params.len() + 1),
                            );
                        params.push(val);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(
                                        format!("{} = ${}::bigint", "granularity", params.len() + 1),
                                    );
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!(
                                            "{} = ANY(${}::bigint[])", "granularity", params.len() + 1
                                        ),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
                        where_clauses
                            .push(
                                format!(
                                    "{} >= ${}::bigint AND {} <= ${}::bigint", "granularity",
                                    params.len() + 1, "granularity", params.len() + 2
                                ),
                            );
                        params.push(start);
                        params.push(end);
                    }
                }
            }
            if let Some(f) = filter.collected_at.as_ref() {
                match f {
                    pco_store::pco_pack::TimeFilter::Equal(v) => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(v);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                        if !vals.is_empty() {
                            where_clauses
                                .push(format!("end_at >= ${}", params.len() + 1));
                            params.push(vals.first().unwrap());
                            where_clauses
                                .push(format!("start_at <= ${}", params.len() + 1));
                            params.push(vals.last().unwrap());
                        }
                    }
                    pco_store::pco_pack::TimeFilter::Range { start, end } => {
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
            let filter_json: pco_store::pco_pack::serde_json::Value = filter
                .clone()
                .try_into()?;
            let fields = <QueryStat as pco_store::pco_pack::PcoPack>::resolve_fields(
                &filter_json,
                fields,
            )?;
            let known: &[&str] = &[
                "database_id",
                "granularity",
                "start_at",
                "end_at",
                "collected_at",
                "calls",
            ];
            let select_columns: Vec<&str> = known
                .iter()
                .copied()
                .filter(|c| fields.iter().any(|f| f == c))
                .collect();
            let query_sql = format!(
                "SELECT {} FROM {} {}", select_columns.join(", "), "query_stats",
                sql_where
            );
            let resolved_fields: Vec<String> = fields
                .iter()
                .map(|f| (*f).to_string())
                .collect();
            let db = self;
            let mut chunks = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks
                    .push(pco_store::Chunk {
                        chunk: ChunkT {
                            database_id: if fields.contains(&"database_id") {
                                let v = row.get::<_, i64>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            granularity: if fields.contains(&"granularity") {
                                let v = row.get::<_, i32>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            start_at: if fields.contains(&"start_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            end_at: if fields.contains(&"end_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            collected_at: if fields.contains(&"collected_at") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                            calls: if fields.contains(&"calls") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                        },
                        filter: filter.clone(),
                        fields: resolved_fields.clone(),
                    });
            }
            Ok(chunks)
        })
    }
    /// Deletes matching rows and returns whole chunks with empty
    /// filters, so decompress answers every row the delete touched and
    /// a re-store loses nothing outside the filter range.
    fn delete<'a>(
        &'a self,
        filter: &'a <QueryStat as pco_store::pco_pack::PcoPack>::Filter,
    ) -> pco_store::BoxFuture<
        'a,
        pco_store::anyhow::Result<Vec<pco_store::Chunk<QueryStat>>>,
    > {
        Box::pin(async move {
            type ChunkT = <QueryStat as pco_store::pco_pack::PcoPack>::Chunk;
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
            if let Some(f) = filter.database_id.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(v) => {
                        where_clauses
                            .push(format!("{} = ${}", "database_id", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(format!("{} = ${}", "database_id", params.len() + 1));
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!("{} = ANY(${})", "database_id", params.len() + 1),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
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
            if let Some(f) = filter.granularity.as_ref() {
                match f {
                    pco_store::pco_pack::I64Filter::Equal(val) => {
                        where_clauses
                            .push(
                                format!("{} = ${}::bigint", "granularity", params.len() + 1),
                            );
                        params.push(val);
                    }
                    pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                        match vals.as_slice() {
                            [] => {}
                            [v] => {
                                where_clauses
                                    .push(
                                        format!("{} = ${}::bigint", "granularity", params.len() + 1),
                                    );
                                params.push(v);
                            }
                            _ => {
                                where_clauses
                                    .push(
                                        format!(
                                            "{} = ANY(${}::bigint[])", "granularity", params.len() + 1
                                        ),
                                    );
                                params.push(vals);
                            }
                        }
                    }
                    pco_store::pco_pack::I64Filter::Range { start, end } => {
                        where_clauses
                            .push(
                                format!(
                                    "{} >= ${}::bigint AND {} <= ${}::bigint", "granularity",
                                    params.len() + 1, "granularity", params.len() + 2
                                ),
                            );
                        params.push(start);
                        params.push(end);
                    }
                }
            }
            if let Some(f) = filter.collected_at.as_ref() {
                match f {
                    pco_store::pco_pack::TimeFilter::Equal(v) => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(v);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(v);
                    }
                    pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                        if !vals.is_empty() {
                            where_clauses
                                .push(format!("end_at >= ${}", params.len() + 1));
                            params.push(vals.first().unwrap());
                            where_clauses
                                .push(format!("start_at <= ${}", params.len() + 1));
                            params.push(vals.last().unwrap());
                        }
                    }
                    pco_store::pco_pack::TimeFilter::Range { start, end } => {
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
            let filter_json: pco_store::pco_pack::serde_json::Value = filter
                .clone()
                .try_into()?;
            let fields = <QueryStat as pco_store::pco_pack::PcoPack>::resolve_fields(
                &filter_json,
                &[],
            )?;
            let known: &[&str] = &[
                "database_id",
                "granularity",
                "start_at",
                "end_at",
                "collected_at",
                "calls",
            ];
            let select_columns: Vec<&str> = known
                .iter()
                .copied()
                .filter(|c| fields.iter().any(|f| f == c))
                .collect();
            let query_sql = format!(
                "DELETE FROM {} {} RETURNING {}", "query_stats", sql_where,
                select_columns.join(", ")
            );
            let resolved_fields: Vec<String> = fields
                .iter()
                .map(|f| (*f).to_string())
                .collect();
            let db = self;
            let mut chunks = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks
                    .push(pco_store::Chunk {
                        chunk: ChunkT {
                            database_id: if fields.contains(&"database_id") {
                                let v = row.get::<_, i64>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            granularity: if fields.contains(&"granularity") {
                                let v = row.get::<_, i32>(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            start_at: if fields.contains(&"start_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            end_at: if fields.contains(&"end_at") {
                                let v = row
                                    .get::<
                                        _,
                                        pco_store::pco_pack::chrono::DateTime<
                                            pco_store::pco_pack::chrono::Utc,
                                        >,
                                    >(index);
                                index += 1;
                                v
                            } else {
                                Default::default()
                            },
                            collected_at: if fields.contains(&"collected_at") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                            calls: if fields.contains(&"calls") {
                                let v = row.get::<_, Option<&[u8]>>(index);
                                index += 1;
                                match v {
                                    Some(b) => {
                                        pco_store::pco_pack::serde_bytes::ByteBuf::from(b)
                                    }
                                    None => Default::default(),
                                }
                            } else {
                                Default::default()
                            },
                        },
                        filter: Default::default(),
                        fields: resolved_fields.clone(),
                    });
            }
            Ok(chunks)
        })
    }
    fn capabilities(&self) -> pco_store::Capabilities {
        pco_store::Capabilities {
            transactions: false,
            rename: false,
            range_get: false,
            sql_pushdown: true,
        }
    }
}
