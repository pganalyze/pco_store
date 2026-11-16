//! A standalone comparison of how pco_store should read the chunk table, and
//! of how much the storage shape and chunk size cost. It uses pco_pack
//! directly and inlines the Postgres operations, so nothing in pco_store's
//! Backend stack colors the measurements.
//!
//! It grew out of a production mystery: EXPLAIN reports ~0.1ms for the chunk
//! load while auto_explain reports multi-second runtimes. The gap lives in
//! phases EXPLAIN never instruments: a plain SELECT detoasts during tuple
//! output, inside the destination receiver, and socket sends block inside the
//! receiver too. On top of that, decoding pco streams on the client dominates
//! the end-to-end wall. The stages that proved this live in git history; what
//! remains is the design question they settled, measured at production scale
//! across two pco_pack chunk sizes:
//!
//! - 32,768 rows (2^15) per stored chunk row versus the pco_pack default of
//!   262,144 (2^18).
//! - One request with `= ANY` arrays (production's current query) versus
//!   splitting the load into one equality query per (database_id, granularity)
//!   index tuple, either spread over pooled connections or pipelined on one
//!   connection. Pipelining alone removes the serialized round trips, so under
//!   a one-thread fold a single connection matches a four-connection pool and
//!   both beat the single `= ANY` request.
//! - Fold parallelism: every strategy also runs under a runtime with four
//!   worker threads. Threads only help where the work already has several
//!   in-flight lanes: the pooled strategy's connections decode concurrently
//!   and cut its wall to roughly a quarter, while ordered single-stream
//!   delivery keeps the `= ANY` and pipelined folds on one thread. A
//!   parallel-decode variant that hands the `= ANY` stream to four worker
//!   tasks recovers most of that scaling, yet still trails the pooled
//!   strategy, which also parallelizes server-side toast fetches across
//!   backends.
//! - One BYTEA per payload field versus one BYTEA per chunk. The single blob
//!   stores fewer bytes (one toast chain per row instead of twelve, less
//!   quarter-page padding) and reads marginally faster, at the cost of no
//!   SQL-level column pruning. pco decode projection survives either shape.
//! - The filesystem backend's shape: a flat directory of whole-chunk files,
//!   one directory listing, then each match read and folded in turn. One
//!   lane mirrors the backend's sequential `read_chunks` stream; a four-lane
//!   variant hands contiguous file groups to blocking threads, which
//!   parallelize reads and decoding on any runtime since blocking threads
//!   are not runtime workers. The OS page cache is a filesystem backend's
//!   only cache, and it stays warm across iterations here just as it does
//!   under the Postgres rows' shared_buffers evictions.
//!
//! Storage is `STORAGE EXTERNAL` everywhere: pco output is already denser
//! than anything Postgres compression could add, so a second pass over the
//! bytes would only add latency.
//!
//! Run against a scratch database (all `pco_store_bs32_*` and
//! `pco_store_bs262_*` tables are dropped and recreated; `CREATE EXTENSION
//! pg_buffercache` must have run there):
//! `cargo bench --features postgres --bench blob_stream`

use std::collections::HashMap;
use std::hint::black_box;
use std::os::unix::fs::MetadataExt;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use futures::TryStreamExt;
use futures::future::join_all;
use markdown_tables::MarkdownTableRow;
use pco_pack::PcoPack;
use pco_pack::serde_bytes::ByteBuf;
use pco_pack::serde_json::json;
use tokio_postgres::Row;
use tokio_postgres::types::{ToSql, Type};

/// One hundred databases over one year: minute and hourly series per
/// database, which lands near a gigabyte of toast at either chunk size. A
/// hundred ids matches the number of databases a production load request
/// carries through `= ANY`.
const DB_COUNT: i64 = 100;
const HOURS: i64 = 8_760;
const GRANULARITIES: [i32; 2] = [60, 3_600];
const TOTAL_ROWS: i64 = DB_COUNT * HOURS * 61;
const ITERATIONS: usize = 3;

/// The synthetic database ids, in production's negative-id space.
fn db_ids() -> Vec<i64> {
    (0..DB_COUNT).map(|k| -666_352_669 - k).collect()
}

/// A fresh pool per runtime: tokio-postgres spawns each connection's driver
/// task on the runtime that created the connection, so a pool built on one
/// runtime must not serve queries from another.
fn build_pool() -> Pool {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost:5432/postgres".into());
    let config = tokio_postgres::Config::from_str(&url).expect("bad DATABASE_URL");
    let manager = Manager::from_config(config, tokio_postgres::NoTls, ManagerConfig { recycling_method: RecyclingMethod::Fast });
    // One connection beyond the four the tuple strategy wants: measure_loads
    // holds a client for the whole session (evictions, sizes), and a pool of
    // four would cap the streamed strategy at three in-flight backends.
    Pool::builder(manager).max_size(5).build().unwrap()
}

const COLUMNS: &str = "database_id bigint NOT NULL, granularity integer NOT NULL, start_at timestamptz NOT NULL, end_at timestamptz NOT NULL";
/// The payload columns of production's `query_stats`, in order.
const PAYLOAD_COLS: &[&str] = &[
    "collected_at",
    "fingerprint",
    "postgres_role_id",
    "toplevel",
    "calls",
    "rows",
    "total_time",
    "io_time",
    "shared_blks_hit",
    "shared_blks_read",
    "temp_blks_read",
    "temp_blks_written",
];
/// The production `query_stats` index set: the single btree over the index
/// columns plus the chunk bounds, so plans here match production plans.
const INDEX: &str = "CREATE INDEX {t}_idx ON {t} USING btree (database_id, end_at, start_at, granularity)";

/// Warm the table then take the best wall time over `ITERATIONS` of the
/// awaited expression. Each iteration starts by evicting every unpinned
/// buffer so the measurement is a cold read against the shared cache, as it
/// is in production.
macro_rules! measure {
    ($db:expr, $e:expr) => {{
        let mut best = f64::MAX;
        for _ in 0..ITERATIONS {
            cold_evict($db).await?;
            let started = Instant::now();
            $e?;
            best = best.min(elapsed(started));
        }
        best
    }};
}

/// A complete chunk-size scenario: the `Stat` type with its pco_pack chunk
/// size, the chunked data model, both storage shapes, the load strategies,
/// and the cold measurement harness. `$attr` is the full `#[pco_pack(...)]`
/// attribute and `$rows_per_chunk` the row batch that fills one stored chunk
/// row; `$prefix` names the scenario's tables.
macro_rules! scenario {
    ($attr:meta, $rows_per_chunk:expr, $prefix:literal) => {
        use super::*;

        const ROWS_PER_CHUNK: usize = $rows_per_chunk;

        /// The filesystem scenario's flat directory of whole-chunk files,
        /// mirroring pco_store's `DurDir`: one file per stored chunk row,
        /// named by table, index values, and bounds.
        pub const FS_ROOT: &str = concat!("target/blob_stream/", $prefix);

        /// Production's wide `QueryStat`: the index columns plus twelve
        /// payload columns, each payload column its own chunk-wide pco
        /// stream. Column names and types match the table DDL, which mirrors
        /// the production Postgres schema (partitioned by granularity, one
        /// btree).
        #[derive(Clone, PcoPack)]
        #[$attr]
        pub struct Stat {
            pub database_id: i64,
            /// Nominal number of seconds the stat represents: 1 minute (60)
            /// or one hour (3600).
            pub granularity: i32,
            pub collected_at: DateTime<Utc>,
            pub fingerprint: i64,
            pub postgres_role_id: i64,
            pub toplevel: bool,
            pub calls: i64,
            pub rows: i64,
            pub total_time: f64,
            pub io_time: f64,
            pub shared_blks_hit: i64,
            pub shared_blks_read: i64,
            pub temp_blks_read: i64,
            pub temp_blks_written: i64,
        }

        /// The chunk type pco_pack generated: index columns and bounds plain,
        /// each payload column a compressed `ByteBuf` payload column.
        type Chunk = <Stat as PcoPack>::Chunk;

        #[derive(Clone, Copy)]
        enum Shape {
            /// pco_store's current shape: one BYTEA per payload column, so
            /// toast stores raw chunk bytes with no Postgres compression
            /// work.
            Cols,
            /// One BYTEA per stored chunk holding pco_pack's whole-chunk
            /// container: fewer toast chains per row, but nothing to prune in
            /// SQL: the whole chunk crosses the wire even when the fold reads
            /// a few columns.
            Blob,
        }

        impl Shape {
            fn table(&self) -> String {
                match self {
                    Shape::Cols => concat!($prefix, "_cols"),
                    Shape::Blob => concat!($prefix, "_blob"),
                }
                .to_string()
            }
            fn payload(&self) -> String {
                match self {
                    Shape::Cols => PAYLOAD_COLS.join(", "),
                    Shape::Blob => "blob".to_string(),
                }
            }
        }

        /// Create and load both shapes, reporting the server version, one
        /// relation size per shape, the allocated bytes of the filesystem
        /// blob directory, and the stored chunk row count. The tables
        /// persist in the database for every later measurement, whichever
        /// runtime created them.
        pub async fn seed(pool: &Pool) -> Result<(String, [String; 2], String, usize)> {
            let db = pool.get().await.context("checkout postgres client")?;
            let version: String = db.query_one("SHOW server_version", &[]).await?.get(0);
            let chunks = build_chunks();
            let mut sizes = [String::new(), String::new()];
            for (size, shape) in std::iter::zip(&mut sizes, [Shape::Cols, Shape::Blob]) {
                let table = shape.table();
                create_table(&db, &shape).await?;
                load_chunks(&db, &shape, &chunks).await?;
                db.execute(&format!("ANALYZE {table}"), &[]).await?;
                let size_bytes: i64 = db
                    .query_one(
                        &format!("SELECT COALESCE(sum(pg_total_relation_size(i.inhrelid)), 0)::bigint FROM pg_inherits i WHERE i.inhparent = '{table}'::regclass"),
                        &[],
                    )
                    .await
                    .context("table size query")?
                    .get(0);
                *size = format!("{} MB", size_bytes / (1024 * 1024));
            }
            // The filesystem backend reads the same chunks as a flat
            // directory of whole-chunk files, named like the generated
            // adapter names them: table, index values, bounds. Recreate it
            // so runs never mix generations.
            let _ = std::fs::remove_dir_all(FS_ROOT);
            std::fs::create_dir_all(FS_ROOT).context("create fs root")?;
            for chunk in &chunks {
                let name = chunk_name(chunk);
                let bytes = <Stat as PcoPack>::to_bytes(std::slice::from_ref(chunk))?;
                std::fs::write(std::path::Path::new(FS_ROOT).join(&name), bytes).with_context(|| format!("write {name}"))?;
            }
            // Allocated bytes of the blob directory: `st_blocks` counts
            // 512-byte units, so this is what the filesystem charges for.
            let mut disk_bytes = 0u64;
            for entry in std::fs::read_dir(FS_ROOT)? {
                disk_bytes += entry?.metadata()?.blocks() * 512;
            }
            Ok((version, sizes, format!("{} MB", disk_bytes / (1024 * 1024)), chunks.len()))
        }

        fn chunk_name(chunk: &Chunk) -> String {
            format!("query_stats-{}-{}-{}-{}.pco", chunk.database_id, chunk.granularity, chunk.start_at.timestamp(), chunk.end_at.timestamp())
        }

        /// pco_store's fs load shape: one directory listing of whole-chunk
        /// files, narrowed by name, each match read and folded. One lane
        /// mirrors the backend's sequential stream; more lanes hand sorted
        /// file groups to `spawn_blocking`, which parallelize on any runtime
        /// because blocking threads are not runtime workers. The bench
        /// filter is the stored window, so name narrowing matches every
        /// file; sorting names keeps lanes even and groups one database's
        /// files together.
        async fn fs_load(lanes: usize) -> Result<usize> {
            let mut names: Vec<std::path::PathBuf> = std::fs::read_dir(FS_ROOT)?.map(|entry| entry.map(|e| e.path())).collect::<Result<_, _>>()?;
            names.sort();
            let mut handles = Vec::new();
            for group in names.chunks(names.len().div_ceil(lanes)) {
                let group = group.to_vec();
                handles.push(tokio::task::spawn_blocking(move || fs_fold(&group)));
            }
            let mut total = 0usize;
            for result in join_all(handles).await {
                total += result.context("fs lane panicked")??;
            }
            Ok(total)
        }

        fn fs_fold(paths: &[std::path::PathBuf]) -> Result<usize> {
            let mut count = 0usize;
            let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
            let mut consumer = pco_pack::fold!(Stat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                count += 1;
                let slot = agg.entry((database_id, fingerprint)).or_default();
                slot.0 += calls;
                slot.1 += total_time;
            });
            let query = json!({});
            for path in paths {
                let bytes = std::fs::read(path)?;
                let chunks = <Stat as PcoPack>::from_bytes(&bytes)?;
                <Stat as PcoPack>::fold_value(&chunks, &query, &[], &mut consumer)?;
            }
            black_box((count, agg.len()));
            Ok(count)
        }

        /// Run every strategy cold against both shapes, returning one cell
        /// per shape per strategy in table order. Decoding and folding
        /// execute on the runtime this is awaited on, so the same function
        /// measures a one-thread fold or a four-thread fold depending on the
        /// runtime; the pool must have been built on that runtime too. The
        /// simple strategy folds inline by default; with `parallel_decode` it
        /// hands each streamed row to four decode workers, which is only
        /// meaningful on a multi-thread runtime.
        pub async fn measure_loads(
            pool: &Pool, db_ids: &[i64], window: (DateTime<Utc>, DateTime<Utc>), parallel_decode: bool,
        ) -> Result<(Vec<[String; 2]>, [String; 2])> {
            let db = pool.get().await.context("checkout postgres client")?;
            let mut strategies: [[String; 2]; 3] = Default::default();
            for (cell, shape) in std::iter::zip(0.., [Shape::Cols, Shape::Blob]) {
                let payload = shape.payload();
                strategies[0][cell] = if parallel_decode {
                    format!("{:.0} ms", measure!(&db, simple_query_parallel_decode(&db, &shape, &payload, db_ids, window).await))
                } else {
                    format!("{:.0} ms", measure!(&db, simple_query(&db, &shape, &payload, db_ids, window).await))
                };
                strategies[1][cell] = format!("{:.0} ms", measure!(&db, tuples_parallel(pool, &shape, &payload, db_ids, window).await));
                strategies[2][cell] = format!("{:.0} ms", measure!(&db, tuples_pipelined(&db, &shape, &payload, db_ids, window).await));
            }
            let fs = [format!("{:.0} ms", measure!(&db, fs_load(1).await)), format!("{:.0} ms", measure!(&db, fs_load(4).await))];
            Ok((strategies.to_vec(), fs))
        }

        /// The single request production issues today: `database_id = ANY($1)
        /// AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4`
        /// over the full stored window, every database id in one array,
        /// served by the production parent index. Rows land through the
        /// projection fold as they stream.
        async fn simple_query(
            db: &deadpool_postgres::Client, shape: &Shape, payload: &str, ids: &[i64], window: (DateTime<Utc>, DateTime<Utc>),
        ) -> Result<usize> {
            let table = shape.table();
            let statement = db
                .prepare_cached(&format!(
                    "SELECT database_id, {payload} FROM {table} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4"
                ))
                .await?;
            let ids = ids.to_vec();
            let granularities = GRANULARITIES.to_vec();
            let params: [&(dyn ToSql + Sync); 4] = [&ids, &granularities, &window.0, &window.1];
            let mut rows = std::pin::pin!(db.query_raw(&statement, params).await?);
            let mut count = 0usize;
            let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
            let mut consumer = pco_pack::fold!(Stat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                count += 1;
                let slot = agg.entry((database_id, fingerprint)).or_default();
                slot.0 += calls;
                slot.1 += total_time;
            });
            while let Some(row) = rows.try_next().await? {
                fold_row(shape, &row, &mut consumer)?;
            }
            black_box((count, agg.len()));
            Ok(count)
        }

        /// The `= ANY` stream with the decode handoff `simple_query` lacks:
        /// each owned row goes to one of four worker tasks over a bounded
        /// channel instead of folding inline, so pco decoding spreads over
        /// four threads even though the socket delivers one ordered stream.
        /// Round-robin placement stays balanced because chunk rows are
        /// uniform; the bounded channels give the producer backpressure, and
        /// workers keep private maps merged only by summed counts.
        async fn simple_query_parallel_decode(
            db: &deadpool_postgres::Client, shape: &Shape, payload: &str, ids: &[i64], window: (DateTime<Utc>, DateTime<Utc>),
        ) -> Result<usize> {
            const WORKERS: usize = 4;
            let table = shape.table();
            let statement = db
                .prepare_cached(&format!(
                    "SELECT database_id, {payload} FROM {table} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4"
                ))
                .await?;
            let ids = ids.to_vec();
            let granularities = GRANULARITIES.to_vec();
            let params: [&(dyn ToSql + Sync); 4] = [&ids, &granularities, &window.0, &window.1];
            let mut rows = std::pin::pin!(db.query_raw(&statement, params).await?);

            let (sinks, mut streams): (Vec<tokio::sync::mpsc::Sender<Row>>, Vec<_>) =
                (0..WORKERS).map(|_| tokio::sync::mpsc::channel::<Row>(8)).unzip();
            let shape = *shape;
            let mut handles = Vec::new();
            for mut rx in streams.drain(..) {
                handles.push(tokio::spawn(async move {
                    let mut count = 0usize;
                    let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
                    let mut consumer = pco_pack::fold!(Stat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                        count += 1;
                        let slot = agg.entry((database_id, fingerprint)).or_default();
                        slot.0 += calls;
                        slot.1 += total_time;
                    });
                    while let Some(row) = rx.recv().await {
                        fold_row(&shape, &row, &mut consumer)?;
                    }
                    black_box((count, agg.len()));
                    Ok::<usize, anyhow::Error>(count)
                }));
            }

            let mut next = 0;
            while let Some(row) = rows.try_next().await? {
                sinks[next].send(row).await.map_err(|_| anyhow::anyhow!("decode worker died"))?;
                next = (next + 1) % WORKERS;
            }
            drop(sinks);

            let mut total = 0usize;
            for result in join_all(handles).await {
                total += result.context("decode worker panicked")??;
            }
            Ok(total)
        }

        /// One streaming request per (database_id, granularity) index tuple,
        /// each with the production equality and range predicates, spread
        /// over pooled connections so server toast fetches keep running while
        /// the client folds earlier chunks.
        async fn tuples_parallel(pool: &Pool, shape: &Shape, payload: &str, ids: &[i64], window: (DateTime<Utc>, DateTime<Utc>)) -> Result<usize> {
            let shape = *shape;
            let table = shape.table();
            let payload = payload.to_string();

            let mut handles = Vec::new();
            for &database_id in ids {
                for granularity in GRANULARITIES {
                    let client = pool.get().await.context("checkout for tuple")?;
                    let (table, payload) = (table.clone(), payload.clone());
                    handles.push(tokio::spawn(async move {
                        let statement = client
                            .prepare_cached(&format!(
                                "SELECT database_id, {payload} FROM {table} WHERE database_id = $1 AND granularity = $2 AND end_at >= $3 AND start_at <= $4"
                            ))
                            .await?;
                        let params: [&(dyn ToSql + Sync); 4] = [&database_id, &granularity, &window.0, &window.1];
                        let mut rows = std::pin::pin!(client.query_raw(&statement, params).await?);
                        let mut count = 0usize;
                        let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
                        let mut consumer = pco_pack::fold!(Stat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                            count += 1;
                            let slot = agg.entry((database_id, fingerprint)).or_default();
                            slot.0 += calls;
                            slot.1 += total_time;
                        });
                        while let Some(row) = rows.try_next().await? {
                            fold_row(&shape, &row, &mut consumer)?;
                        }
                        black_box((count, agg.len()));
                        Ok::<usize, anyhow::Error>(count)
                    }));
                }
            }

            let mut total = 0usize;
            for result in join_all(handles).await {
                total += result.context("tuple task panicked")??;
            }
            Ok(total)
        }

        /// The same per-tuple requests as `tuples_parallel`, but all on one
        /// connection: tokio-postgres pipelines the requests onto one socket
        /// and delivers responses in order. Submission and row consumption
        /// must interleave: the connection task dispatches responses to the
        /// head query, so a design that submits everything before consuming
        /// anything stalls the socket the moment the head query's rows wait
        /// for a consumer. `join_all` interleaves the futures on this task,
        /// which is the documented pipelining pattern without the `Send`
        /// requirement (client handles are not `Clone`).
        async fn tuples_pipelined(
            db: &deadpool_postgres::Client, shape: &Shape, payload: &str, ids: &[i64], window: (DateTime<Utc>, DateTime<Utc>),
        ) -> Result<usize> {
            let table = shape.table();
            let statement = db
                .prepare_cached(&format!(
                    "SELECT database_id, {payload} FROM {table} WHERE database_id = $1 AND granularity = $2 AND end_at >= $3 AND start_at <= $4"
                ))
                .await?;
            let tuples: Vec<(i64, i32)> = ids.iter().flat_map(|&id| GRANULARITIES.map(|g| (id, g))).collect();

            let requests = tuples.iter().map(|&(database_id, granularity)| {
                let (client, statement) = (db, statement.clone());
                async move {
                    let params: [&(dyn ToSql + Sync); 4] = [&database_id, &granularity, &window.0, &window.1];
                    let mut rows = std::pin::pin!(client.query_raw(&statement, params).await?);
                    let mut count = 0usize;
                    let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
                    let mut consumer = pco_pack::fold!(Stat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                        count += 1;
                        let slot = agg.entry((database_id, fingerprint)).or_default();
                        slot.0 += calls;
                        slot.1 += total_time;
                    });
                    while let Some(row) = rows.try_next().await? {
                        fold_row(shape, &row, &mut consumer)?;
                    }
                    black_box((count, agg.len()));
                    Ok::<usize, anyhow::Error>(count)
                }
            });

            let mut total = 0usize;
            for result in join_all(requests.collect::<Vec<_>>()).await {
                total += result?;
            }
            Ok(total)
        }

        /// Feed one result row's payload through the fold engine. Bounds
        /// carry defaults because no fold decodes them; `database_id` stays
        /// live, since the projection fold keys its running sums by it.
        fn fold_row<C: pco_pack::Consumer<Stat>>(shape: &Shape, row: &Row, consumer: &mut C) -> Result<()> {
            let query = json!({});
            match shape {
                Shape::Blob => {
                    let chunks = <Stat as PcoPack>::from_bytes(row.get::<_, &[u8]>(1))?;
                    <Stat as PcoPack>::fold_value(&chunks, &query, &[], consumer)?;
                }
                Shape::Cols => {
                    let database_id: i64 = row.get(0);
                    let chunk = Chunk {
                        database_id,
                        granularity: 0,
                        start_at: DateTime::<Utc>::UNIX_EPOCH,
                        end_at: DateTime::<Utc>::UNIX_EPOCH,
                        collected_at: ByteBuf::from(row.get::<_, &[u8]>(1)),
                        fingerprint: ByteBuf::from(row.get::<_, &[u8]>(2)),
                        postgres_role_id: ByteBuf::from(row.get::<_, &[u8]>(3)),
                        toplevel: ByteBuf::from(row.get::<_, &[u8]>(4)),
                        calls: ByteBuf::from(row.get::<_, &[u8]>(5)),
                        rows: ByteBuf::from(row.get::<_, &[u8]>(6)),
                        total_time: ByteBuf::from(row.get::<_, &[u8]>(7)),
                        io_time: ByteBuf::from(row.get::<_, &[u8]>(8)),
                        shared_blks_hit: ByteBuf::from(row.get::<_, &[u8]>(9)),
                        shared_blks_read: ByteBuf::from(row.get::<_, &[u8]>(10)),
                        temp_blks_read: ByteBuf::from(row.get::<_, &[u8]>(11)),
                        temp_blks_written: ByteBuf::from(row.get::<_, &[u8]>(12)),
                    };
                    <Stat as PcoPack>::fold_value(std::slice::from_ref(&chunk), &query, &[], consumer)?;
                }
            }
            Ok(())
        }

        async fn create_table(db: &deadpool_postgres::Client, shape: &Shape) -> Result<()> {
            let table = shape.table();
            db.execute(&format!("DROP TABLE IF EXISTS {table} CASCADE"), &[]).await?;
            let payload_ddl = if matches!(shape, Shape::Blob) {
                "blob bytea NOT NULL".to_string()
            } else {
                PAYLOAD_COLS.iter().map(|c| format!("{c} bytea NOT NULL")).collect::<Vec<_>>().join(", ")
            };
            let storage: String = match shape {
                Shape::Cols => PAYLOAD_COLS.iter().map(|c| format!("ALTER TABLE {table} ALTER COLUMN {c} SET STORAGE EXTERNAL; ")).collect(),
                Shape::Blob => format!("ALTER TABLE {table} ALTER COLUMN blob SET STORAGE EXTERNAL; "),
            };
            let ddl = format!(
                "CREATE TABLE {table} ({COLUMNS}, {payload_ddl}) PARTITION BY LIST (granularity); \
                 CREATE TABLE {table}_60 PARTITION OF {table} FOR VALUES IN (60); \
                 CREATE TABLE {table}_3600 PARTITION OF {table} FOR VALUES IN (3600); \
                 {storage}{}",
                INDEX.replace("{t}", &table)
            );
            db.simple_query(&ddl).await.with_context(|| format!("create table: {ddl}"))?;
            Ok(())
        }

        async fn load_chunks(db: &deadpool_postgres::Client, shape: &Shape, chunks: &[Chunk]) -> Result<()> {
            let table = shape.table();
            db.execute(&format!("DELETE FROM {table}"), &[]).await?;
            let (columns, types): (String, Vec<Type>) = match shape {
                Shape::Blob => (
                    "database_id, granularity, start_at, end_at, blob".into(),
                    vec![Type::INT8, Type::INT4, Type::TIMESTAMPTZ, Type::TIMESTAMPTZ, Type::BYTEA],
                ),
                Shape::Cols => (
                    format!("database_id, granularity, start_at, end_at, {}", PAYLOAD_COLS.join(", ")),
                    std::iter::once(Type::INT8)
                        .chain(std::iter::once(Type::INT4))
                        .chain(std::iter::once(Type::TIMESTAMPTZ))
                        .chain(std::iter::once(Type::TIMESTAMPTZ))
                        .chain(PAYLOAD_COLS.iter().map(|_| Type::BYTEA))
                        .collect(),
                ),
            };
            let statement = db.prepare_cached(&format!("COPY {table} ({columns}) FROM STDIN BINARY")).await?;
            let mut writer = std::pin::pin!(tokio_postgres::binary_copy::BinaryCopyInWriter::new(db.copy_in(&statement).await?, &types));
            for chunk in chunks {
                match shape {
                    Shape::Blob => {
                        let blob = <Stat as PcoPack>::to_bytes(std::slice::from_ref(chunk))?;
                        writer.as_mut().write(&[&chunk.database_id, &chunk.granularity, &chunk.start_at, &chunk.end_at, &blob.as_slice()]).await?;
                    }
                    Shape::Cols => {
                        writer
                            .as_mut()
                            .write(&[
                                &chunk.database_id,
                                &chunk.granularity,
                                &chunk.start_at,
                                &chunk.end_at,
                                &chunk.collected_at.as_slice(),
                                &chunk.fingerprint.as_slice(),
                                &chunk.postgres_role_id.as_slice(),
                                &chunk.toplevel.as_slice(),
                                &chunk.calls.as_slice(),
                                &chunk.rows.as_slice(),
                                &chunk.total_time.as_slice(),
                                &chunk.io_time.as_slice(),
                                &chunk.shared_blks_hit.as_slice(),
                                &chunk.shared_blks_read.as_slice(),
                                &chunk.temp_blks_read.as_slice(),
                                &chunk.temp_blks_written.as_slice(),
                            ])
                            .await?;
                    }
                }
            }
            writer.finish().await?;
            Ok(())
        }

        /// The stored chunk rows: per database and granularity, one packed
        /// chunk per `ROWS_PER_CHUNK` rows over `HOURS`. The one-row-per-hour
        /// hourly series is the production shape; the minute series carries
        /// the volume.
        fn build_chunks() -> Vec<Chunk> {
            let base = base();
            let mut chunks = Vec::new();
            for database_id in db_ids() {
                for granularity in GRANULARITIES {
                    let series_rows = HOURS * if granularity == 60 { 60 } else { 1 };
                    let mut series: Vec<Stat> = Vec::with_capacity(ROWS_PER_CHUNK);
                    for i in 0..series_rows {
                        series.push(stat(database_id, granularity, base, i));
                        if series.len() == ROWS_PER_CHUNK || i == series_rows - 1 {
                            chunks.extend(<Stat as PcoPack>::write(std::mem::take(&mut series)).expect("write chunks"));
                            series = Vec::with_capacity(ROWS_PER_CHUNK);
                        }
                    }
                }
            }
            chunks
        }

        fn stat(database_id: i64, granularity: i32, base: DateTime<Utc>, i: i64) -> Stat {
            let seed = (i as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ ((database_id as u64) << 17) ^ (granularity as u64);
            let m = |k: u64| splitmix64(seed ^ k.wrapping_mul(0xD6E8FEB86659FD93));
            let calls = (m(4) % 4096) as i64 + 1;
            Stat {
                database_id,
                granularity,
                collected_at: base + chrono::Duration::seconds(i * granularity as i64),
                fingerprint: splitmix64(m(1) % 5000) as i64,
                postgres_role_id: (m(2) % 20) as i64 + 1,
                toplevel: m(3) % 8 == 0,
                calls,
                rows: (m(5) % 100_000) as i64 + 1,
                total_time: calls as f64 * 0.8 + (m(6) % 100) as f64 / 100.0,
                io_time: if m(7) % 13 == 0 { calls as f64 * 0.09 } else { 0.0 },
                shared_blks_hit: (m(8) % 4096) as i64,
                shared_blks_read: (m(9) % 64) as i64,
                temp_blks_read: if m(10) % 7 == 0 { (m(11) % 32) as i64 } else { 0 },
                temp_blks_written: if m(12) % 9 == 0 { (m(13) % 32) as i64 } else { 0 },
            }
        }
    };
}

/// 2^15 rows per stored chunk row.
mod chunk32k {
    scenario!(pco_pack(timestamp = collected_at, index = [database_id, granularity], chunk_size = 32_768, float_round = 2), 32_768, "pco_store_bs32");
}

/// The pco_pack default, 2^18 rows per stored chunk row: the attribute omits
/// `chunk_size`, so the scenario tracks whatever `PcoPack::CHUNK_SIZE` says.
mod chunk262k {
    scenario!(pco_pack(timestamp = collected_at, index = [database_id, granularity], float_round = 2), 262_144, "pco_store_bs262");
}

/// One report table row: a metric name and two measured strings, one per
/// shape.
struct MetricRow {
    metric: String,
    cells: Vec<String>,
}

impl MarkdownTableRow for MetricRow {
    fn column_names() -> Vec<&'static str> {
        vec!["Metric", "blob per field", "single blob"]
    }
    fn column_values(&self) -> Vec<String> {
        let mut values = vec![self.metric.clone()];
        values.extend(self.cells.iter().cloned());
        values
    }
}

/// One filesystem report table row: a metric name and one measured string.
/// The backend has a single whole-chunk layout, so its table needs no shape
/// columns.
struct FsRow {
    metric: String,
    cells: [String; 1],
}

impl MarkdownTableRow for FsRow {
    fn column_names() -> Vec<&'static str> {
        vec!["Metric", "filesystem blobs"]
    }
    fn column_values(&self) -> Vec<String> {
        let mut values = vec![self.metric.clone()];
        values.extend(self.cells.iter().cloned());
        values
    }
}

fn main() -> Result<()> {
    let db_ids = db_ids();
    // The full stored window; every strategy asks for exactly this range.
    let window = (base(), base() + chrono::Duration::hours(HOURS));

    // A current-thread runtime first: every fold runs on one thread, the
    // floor for a backend that handles a single request at a time.
    let single = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let (version, seed32, pg_single32, fs_single32, seed262, pg_single262, fs_single262) = single.block_on(async {
        let pool = build_pool();
        let (version, seed32, cells32) = {
            let (version, sizes, disk, rows) = chunk32k::seed(&pool).await?;
            let cells = chunk32k::measure_loads(&pool, &db_ids, window, false).await?;
            (version, (sizes, disk, rows), cells)
        };
        let (seed262, cells262) = {
            let (_, sizes, disk, rows) = chunk262k::seed(&pool).await?;
            let cells = chunk262k::measure_loads(&pool, &db_ids, window, false).await?;
            ((sizes, disk, rows), cells)
        };
        Ok::<_, anyhow::Error>((version, seed32, cells32.0, cells32.1, seed262, cells262.0, cells262.1))
    })?;

    // Then a four-worker runtime, where folds from different connections
    // land on different OS threads. The simple strategy switches to its
    // parallel-decode variant here: handoffs to workers need real threads.
    let quad = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build()?;
    let (pg_quad32, fs_quad32) = quad.block_on(async { chunk32k::measure_loads(&build_pool(), &db_ids, window, true).await })?;
    let (pg_quad262, fs_quad262) = quad.block_on(async { chunk262k::measure_loads(&build_pool(), &db_ids, window, true).await })?;

    println!(
        "Postgres {version}. {TOTAL_ROWS} rows ({DB_COUNT} databases x one year). The per-field shape stores one BYTEA per payload column, the single blob shape one BYTEA per chunk; both STORAGE EXTERNAL, LIST-partitioned on granularity, single production btree. Every measurement evicts unpinned buffers first and reports the best of {ITERATIONS}.\n"
    );
    println!(
        "Load: database_id = ANY({DB_COUNT} ids), granularity = ANY(60, 3600), the full stored window, folded \
         through the projection fold as rows arrive. Simple is one request, folded inline under one thread and \
         handed to four decode workers under four threads. The other two split it into one equality query per \
         (database_id, granularity) index tuple, measured under both runtimes. Filesystem reads the same chunks \
         as one file per chunk row from a flat directory, listing once and folding each file; lanes are blocking \
         threads, so both runtimes report them.\n"
    );

    for (label, (pg_sizes, fs_size, chunk_rows), pg_single, fs_single, pg_quad, fs_quad) in [
        ("32,768 rows (2^15)", seed32, pg_single32, fs_single32, pg_quad32, fs_quad32),
        ("262,144 rows (2^18, the pco_pack default)", seed262, pg_single262, fs_single262, pg_quad262, fs_quad262),
    ] {
        println!("Chunk size {label}: {chunk_rows} stored chunk rows per shape\n");

        let mut pg_rows = vec![MetricRow { metric: "Table size".to_string(), cells: pg_sizes.to_vec() }];
        let passes = [
            (["1 thread", "4 conn, 1 thread", "1 conn, 1 thread"], &pg_single),
            (["parallel decode, 4 threads", "4 conn, 4 threads", "1 conn, 4 threads"], &pg_quad),
        ];
        for (descriptors, cells) in passes {
            for (row, (base, descriptor)) in std::iter::zip(cells, std::iter::zip(["Simple query", "Streamed query", "Pipelined query"], descriptors))
            {
                pg_rows.push(MetricRow { metric: format!("{base}: {descriptor}"), cells: row.to_vec() });
            }
        }
        println!("Postgres\n{}", markdown_tables::as_table(&pg_rows));

        let mut fs_rows = vec![FsRow { metric: "Disk size".to_string(), cells: [fs_size] }];
        for (threads, fs) in [("1 thread", fs_single), ("4 threads", fs_quad)] {
            for (cell, lanes) in std::iter::zip(fs, ["1 lane", "4 lanes"]) {
                fs_rows.push(FsRow { metric: format!("{lanes}, {threads}"), cells: [cell] });
            }
        }
        println!("Filesystem\n{}", markdown_tables::as_table(&fs_rows));
    }
    Ok(())
}

fn elapsed(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

/// Seed time for every series.
fn base() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2021, 11, 13, 0, 0, 0).unwrap()
}

/// splitmix64: full entropy per call, so pco pays full width on whichever
/// columns use it. An LCG over consecutive inputs has a constant difference
/// and delta-codes away to nothing, which quietly made an earlier payload
/// compressible.
fn splitmix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Evict every unpinned buffer, so a measurement starts with the shared
/// cache as empty as Postgres allows. `pg_buffercache_evict` is superuser-
/// only and needs the extension; a scratch database gets both.
async fn cold_evict(db: &deadpool_postgres::Client) -> Result<()> {
    db.query_one(
        "SELECT count(*) FROM pg_buffercache WHERE relfilenode IS NOT NULL AND pinning_backends = 0 AND pg_buffercache_evict(bufferid)",
        &[],
    )
    .await
    .context("cold eviction needs the pg_buffercache extension (CREATE EXTENSION pg_buffercache) and superuser")?;
    Ok(())
}
