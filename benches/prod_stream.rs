//! Measure production `query_stats` read patterns against a live database.
//!
//! Unlike `blob_stream`, which synthesizes its own tables, this benchmark
//! runs against an existing `query_stats` table in whatever database
//! `DATABASE_URL` points at, so the plans, toast chains, index layout, and
//! (on RDS) network and IO latency are the real ones. The Postgres operation
//! shapes match `blob_stream` exactly: one `= ANY` request; one equality
//! request per (database_id, granularity) index tuple spread over a small
//! connection pool; the same per-tuple requests pipelined onto one
//! connection; and the `= ANY` stream handed to parallel decode workers.
//! Every row lands through the projection fold, keyed by
//! (database_id, fingerprint) with running sums of (calls, total_time).
//!
//! The bytes are decoded through pco_pack's current codec, which is what
//! pco_store writes today. If the table predates that codec, decoding errors
//! name the row and phase instead of guessing.
//!
//! `--backfill` creates a sibling single-blob table (one BYTEA per chunk
//! row holding pco_pack's whole-chunk container) partitioned and indexed
//! like the source, fills it from the same window, and then the report
//! compares both shapes side by side. Backfill is a pure stream transform:
//! each source row's twelve field blobs move into one container with
//! `to_bytes`, so the only CPU spent is container serialization.
//!
//! Connections follow the URL's `sslmode`: RDS endpoints need TLS, which
//! `postgres-native-tls` supplies. Buffer eviction (`--evict`) needs
//! pg_buffercache and superuser, so it stays off by default; RDS cannot
//! offer it. Without eviction every cell still reports best and first
//! iteration, so the first iteration shows whatever IO the operating system
//! and RDS had not already cached.
//!
//! Example:
//! ```text
//! DATABASE_URL=postgres://user@prod/db cargo bench --features postgres \
//!   --bench prod_stream -- \
//!   --db-ids -666352669,-666352670 --granularities 60,3600 \
//!   --from 2026-11-05T15:00:00Z --to 2026-11-12T13:00:00Z --backfill
//! ```

use std::collections::HashMap;
use std::hint::black_box;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDate, Utc};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use futures::TryStreamExt;
use futures::future::join_all;
use pco_pack::PcoPack;
use pco_pack::serde_bytes::ByteBuf;
use pco_pack::serde_json::json;
use tokio::sync::mpsc;
use tokio_postgres::Row;
use tokio_postgres::types::{ToSql, Type};

/// The payload columns of production's `query_stats`, in table order. The
/// index columns (`database_id`, `granularity`) and the chunk bounds
/// (`start_at`, `end_at`) stay plain; everything else is one BYTEA per
/// column holding that column's chunk-wide pco stream.
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

/// Production's `QueryStat`: the same derive `pco_store` emits its backend
/// from, so the chunk layout decoded here matches the bytes in the table.
/// Neither `chunk_size` nor `float_round` appears: this benchmark only ever
/// decodes or re-bundles stored streams, never encodes new ones.
#[derive(Clone, pco_pack::PcoPack)]
#[pco_pack(timestamp = collected_at, index = [database_id, granularity])]
pub struct QueryStat {
    pub database_id: i64,
    /// Nominal number of seconds the stat represents: 1 minute (60) or one
    /// hour (3600).
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

type Chunk = <QueryStat as PcoPack>::Chunk;

/// Which physical layout to measure. The source table is always the
/// blob-per-field layout production already has; the single blob layout only
/// exists after `--backfill` (or when the user points `--blob-table` at a
/// table they backfilled earlier).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeKind {
    Cols,
    Blob,
}

/// A measured layout: its kind plus the concrete table to read.
#[derive(Clone)]
struct Shape {
    kind: ShapeKind,
    table: String,
}

impl Shape {
    fn label(&self) -> &'static str {
        match self.kind {
            ShapeKind::Cols => "blob per field",
            ShapeKind::Blob => "single blob",
        }
    }
    /// The SELECT payload list behind the `database_id` column: every field
    /// blob for the per-field layout, the one container blob for the single
    /// blob layout.
    fn payload(&self) -> String {
        match self.kind {
            ShapeKind::Cols => PAYLOAD_COLS.join(", "),
            ShapeKind::Blob => "blob".to_string(),
        }
    }
}

/// Everything the command line says: which rows to read, which tables hold
/// them, and how hard to push.
struct Args {
    db_ids: Vec<i64>,
    granularities: Vec<i32>,
    window: (DateTime<Utc>, DateTime<Utc>),
    /// The existing blob-per-field table.
    table: String,
    /// The single-blob sibling, created by `--backfill`.
    blob_table: String,
    backfill: bool,
    shapes: Vec<ShapeKind>,
    /// Connections the streamed strategy may hold at once.
    conns: usize,
    /// Worker threads on the parallel runtime, and decode workers the
    /// `= ANY` handoff variant gets under it.
    threads: usize,
    iterations: usize,
    evict: bool,
}

const USAGE: &str = "usage: prod_stream --db-ids <ids> --from <ts> --to <ts> [options]

required:
  --db-ids <csv>            database ids to load, comma separated
  --from <ts>               window start: RFC 3339 or YYYY-MM-DD (UTC)
  --to <ts>                 window end, inclusive: same formats

options:
  --granularities <csv>     default 60,3600
  --table <name>            existing blob-per-field table, default query_stats
  --blob-table <name>       single-blob table, default <table>_blob
  --backfill                recreate and fill the blob table from the source
  --shapes <list>           cols, blob, or cols,blob; default cols (cols,blob with --backfill)
  --conns <n>               pooled connections for the streamed strategy, default 4
  --threads <n>             parallel runtime workers and decode workers, default 4
  --iterations <n>          timed repetitions per strategy, default 3
  --evict                   evict unpinned buffers before each iteration
                            (needs pg_buffercache and superuser; RDS cannot)

DATABASE_URL is required and points at the production database.";

impl Args {
    fn parse() -> Result<Args> {
        let mut db_ids: Option<Vec<i64>> = None;
        let mut granularities = Some(vec![60, 3_600]);
        let mut from: Option<DateTime<Utc>> = None;
        let mut to: Option<DateTime<Utc>> = None;
        let mut table = "query_stats".to_string();
        let mut blob_table: Option<String> = None;
        let mut backfill = false;
        let mut shapes: Option<Vec<ShapeKind>> = None;
        let mut conns = 4usize;
        let mut threads = 4usize;
        let mut iterations = 3usize;
        let mut evict = false;

        let mut argv = std::env::args().skip(1);
        while let Some(flag) = argv.next() {
            // Every value-taking flag shares this accessor so a trailing
            // flag with no value reads as the same error everywhere.
            let mut value = |what: &str| -> Result<String> { argv.next().with_context(|| format!("{what} needs a value")) };
            match flag.as_str() {
                "--help" | "-h" => {
                    println!("{USAGE}");
                    std::process::exit(0);
                }
                // `cargo bench` appends a bare `--bench` to harness-free benches.
                "--bench" => {}
                "--db-ids" => db_ids = Some(parse_csv::<i64>(&value("--db-ids")?, "--db-ids")?),
                "--granularities" => granularities = Some(parse_csv::<i32>(&value("--granularities")?, "--granularities")?),
                "--from" => from = Some(parse_time(&value("--from")?, "--from")?),
                "--to" => to = Some(parse_time(&value("--to")?, "--to")?),
                "--table" => table = value("--table")?,
                "--blob-table" => blob_table = Some(value("--blob-table")?),
                "--backfill" => backfill = true,
                "--shapes" => {
                    let mut kinds = Vec::new();
                    for name in parse_csv::<String>(&value("--shapes")?, "--shapes")? {
                        kinds.push(match name.trim() {
                            "cols" => ShapeKind::Cols,
                            "blob" => ShapeKind::Blob,
                            other => bail!("unknown shape {other:?}; expected cols or blob"),
                        });
                    }
                    shapes = Some(kinds);
                }
                "--conns" => conns = value("--conns")?.parse().context("--conns wants a positive integer")?,
                "--threads" => threads = value("--threads")?.parse().context("--threads wants a positive integer")?,
                "--iterations" => iterations = value("--iterations")?.parse().context("--iterations wants a positive integer")?,
                "--evict" => evict = true,
                other => bail!("unknown argument {other:?}\n\n{USAGE}"),
            }
        }

        let db_ids = db_ids.context("--db-ids is required\n\n{USAGE}")?;
        let (from, to) = (from.context("--from is required\n\n{USAGE}")?, to.context("--to is required\n\n{USAGE}")?);
        anyhow::ensure!(!db_ids.is_empty(), "--db-ids named no ids");
        anyhow::ensure!(from < to, "--from must precede --to");
        let granularities = granularities.expect("defaulted above");
        anyhow::ensure!(!granularities.is_empty(), "--granularities named no values");
        anyhow::ensure!(conns >= 1 && threads >= 1 && iterations >= 1, "--conns, --threads, and --iterations must be positive");

        let shapes = shapes.unwrap_or(if backfill { vec![ShapeKind::Cols, ShapeKind::Blob] } else { vec![ShapeKind::Cols] });
        let blob_table = blob_table.unwrap_or_else(|| format!("{table}_blob"));
        anyhow::ensure!(
            !(backfill && !shapes.contains(&ShapeKind::Blob)),
            "--backfill is pointless without the blob shape; drop it or add blob to --shapes"
        );

        Ok(Args { db_ids, granularities, window: (from, to), table, blob_table, backfill, shapes, conns, threads, iterations, evict })
    }

    /// The tables to report on, source layout first.
    fn shapes(&self) -> Vec<Shape> {
        self.shapes
            .iter()
            .map(|kind| Shape {
                kind: *kind,
                table: match kind {
                    ShapeKind::Cols => self.table.clone(),
                    ShapeKind::Blob => self.blob_table.clone(),
                },
            })
            .collect()
    }
}

fn parse_csv<T>(text: &str, what: &str) -> Result<Vec<T>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    text.split(',').map(|part| part.trim().parse::<T>().with_context(|| format!("{what} contains an invalid value {part:?}"))).collect()
}

fn parse_time(text: &str, what: &str) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
        return Ok(parsed.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).expect("midnight").and_utc());
    }
    bail!("{what} wants an RFC 3339 timestamp or a YYYY-MM-DD date, got {text:?}")
}

/// A fresh pool per runtime: tokio-postgres spawns each connection's driver
/// task on the runtime that created the connection, so a pool built on one
/// runtime must not serve queries from another. TLS follows the URL's
/// `sslmode`: production endpoints need it, a local `sslmode=disable` URL
/// skips it, and `prefer` negotiates either way.
fn build_pool(max_size: usize) -> Result<Pool> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is required: it points at the production database")?;
    let config = tokio_postgres::Config::from_str(&url).context("DATABASE_URL does not parse")?;
    let connector = postgres_native_tls::MakeTlsConnector::new(native_tls::TlsConnector::builder().build().context("build TLS connector")?);
    let manager = Manager::from_config(config, connector, ManagerConfig { recycling_method: RecyclingMethod::Fast });
    Ok(Pool::builder(manager).max_size(max_size).build().context("build pool")?)
}

/// Best and first wall time in ms over `--iterations` of the awaited
/// expression, each iteration optionally starting with a full buffer
/// eviction. The first iteration is the interesting one against RDS: it is
/// where any IO the cache had not already absorbed shows up.
macro_rules! measure {
    ($db:expr, $args:expr, $e:expr) => {{
        let mut best = f64::MAX;
        let mut first = 0.0f64;
        for iteration in 0..$args.iterations {
            if $args.evict {
                cold_evict($db).await?;
            }
            let started = Instant::now();
            $e?;
            let ms = elapsed(started);
            if iteration == 0 {
                first = ms;
            }
            best = best.min(ms);
        }
        (best, first)
    }};
}

fn cell(pair: (f64, f64)) -> String {
    format!("{:.0} / {:.0}", pair.0, pair.1)
}

fn elapsed(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    let shapes = args.shapes();

    // A current-thread runtime first: one-thread folds, and every
    // preparation (existence checks, backfill, sizes, decode verification)
    // so the timed passes touch nothing but the measurement itself.
    let single = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let (version, prep, single_cells) = single.block_on(async {
        let pool = build_pool(args.conns + 1)?;
        let prep = prepare(&pool, &args).await?;
        let cells = measure_loads(&pool, &args, &shapes, false).await?;
        let version: String = pool.get().await?.query_one("SHOW server_version", &[]).await?.get(0);
        Ok::<_, anyhow::Error>((version, prep, cells))
    })?;

    // Then the parallel runtime: decoded folds land on separate OS threads.
    // The simple strategy switches to its parallel-decode variant here; the
    // streamed strategy simply holds its connections longer than the fold
    // needs, which is exactly why it scales.
    let quad = tokio::runtime::Builder::new_multi_thread().worker_threads(args.threads).enable_all().build()?;
    let quad_cells = quad.block_on(async {
        let pool = build_pool(args.conns + 1)?;
        measure_loads(&pool, &args, &shapes, true).await
    })?;

    let (from, to) = args.window;
    println!(
        "Postgres {version}. Load: database_id = ANY({} ids), granularity = ANY({}), end_at >= {from} AND start_at <= {to} \
         over table {}, folded through the projection fold as rows arrive. One column per storage shape{}. \
         Every cell is best / first iteration ms over {} iteration(s); first is the cold read.\n",
        args.db_ids.len(),
        args.granularities.iter().map(i32::to_string).collect::<Vec<_>>().join(", "),
        args.table,
        if shapes.len() > 1 { ", the source compared against the backfilled single-blob table" } else { "" },
        args.iterations,
    );
    println!(
        "Simple is one request, folded inline on one thread and handed to {} decode workers on the parallel runtime. \
         Streamed splits the load into one equality query per (database_id, granularity) index tuple over {} pooled \
         connections; pipelined runs the same tuples on one connection.\n",
        args.threads, args.conns,
    );

    let mut header = vec!["Metric".to_string()];
    header.extend(shapes.iter().map(|shape| shape.label().to_string()));
    let mut rows: Vec<Vec<String>> = Vec::new();
    rows.push(std::iter::once("Table size".to_string()).chain(prep.sizes.clone()).collect());
    rows.push(std::iter::once("Chunk rows in window".to_string()).chain(prep.counts.iter().map(usize::to_string)).collect());
    let passes = [
        (["1 thread", &format!("{} conn, 1 thread", args.conns)[..], "1 conn, 1 thread"], &single_cells),
        (
            [
                &format!("parallel decode, {} threads", args.threads)[..],
                &format!("{} conn, {} threads", args.conns, args.threads)[..],
                &format!("1 conn, {} threads", args.threads)[..],
            ],
            &quad_cells,
        ),
    ];
    for (descriptors, cells) in passes {
        for (row, (base, descriptor)) in std::iter::zip(cells, std::iter::zip(["Simple query", "Streamed query", "Pipelined query"], descriptors)) {
            rows.push(std::iter::once(format!("{base}: {descriptor}")).chain(row.clone()).collect());
        }
    }
    println!("{}", markdown_table(&header, &rows));
    Ok(())
}

/// What preparation learned about each shape: relation size and window row
/// count, in table-report order.
struct Prep {
    sizes: Vec<String>,
    counts: Vec<usize>,
}

/// Check the tables exist, backfill the blob table if asked, then read the
/// sizes and fold one window row per shape so a codec mismatch fails before
/// any timing work starts.
async fn prepare(pool: &Pool, args: &Args) -> Result<Prep> {
    let db = pool.get().await?;
    ensure_table(&db, &args.table, "the source table").await?;
    if args.backfill {
        backfill(pool, args).await?;
    } else if args.shapes.contains(&ShapeKind::Blob) {
        ensure_table(&db, &args.blob_table, "the blob table (re-run with --backfill to create it)").await?;
    }

    let (from, to) = args.window;
    let mut sizes = Vec::new();
    let mut counts = Vec::new();
    for shape in args.shapes() {
        let bytes: i64 = db
            .query_one(
                &format!(
                    "SELECT (CASE WHEN (SELECT count(*) FROM pg_inherits WHERE inhparent = '{0}'::regclass) > 0 \
                     THEN (SELECT COALESCE(sum(pg_total_relation_size(i.inhrelid)), 0) FROM pg_inherits i WHERE i.inhparent = '{0}'::regclass) \
                     ELSE pg_total_relation_size('{0}'::regclass) END)::bigint",
                    shape.table
                ),
                &[],
            )
            .await
            .with_context(|| format!("size of {}", shape.table))?
            .get(0);
        sizes.push(format!("{} MB", bytes / (1024 * 1024)));
        let count: i64 = db
            .query_one(
                &format!(
                    "SELECT count(*) FROM {0} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4",
                    shape.table
                ),
                &[&args.db_ids, &args.granularities, &from, &to],
            )
            .await
            .with_context(|| format!("row count of {}", shape.table))?
            .get(0);
        anyhow::ensure!(count > 0, "{} selects no chunk rows for this window", shape.table);
        counts.push(count as usize);

        // One row through the fold: if the stored bytes are not pco_pack's
        // current per-field format, this is where we learn it.
        let statement = db
            .prepare_cached(&format!(
                "SELECT database_id, {} FROM {} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4 LIMIT 1",
                shape.payload(),
                shape.table
            ))
            .await?;
        let rows = db.query(&statement, &[&args.db_ids, &args.granularities, &from, &to]).await?;
        let mut count = 0usize;
        let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
        let mut consumer = pco_pack::fold!(QueryStat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
            count += 1;
            let slot = agg.entry((database_id, fingerprint)).or_default();
            slot.0 += calls;
            slot.1 += total_time;
        });
        for row in &rows {
            fold_row(shape.kind, row, &mut consumer)
                .with_context(|| format!("decoding a chunk row from {} failed: these bytes may predate pco_pack's current codec", shape.table))?;
        }
        anyhow::ensure!(count > 0, "the first chunk row of {} decoded no logical rows", shape.table);
        println!("verified {} by folding one chunk row: {count} logical rows", shape.table);
    }
    Ok(Prep { sizes, counts })
}

/// Fail with a readable message rather than a SQL error deep inside a
/// strategy, when a named table is not there.
async fn ensure_table(db: &deadpool_postgres::Client, table: &str, role: &str) -> Result<()> {
    let exists: bool = db.query_one("SELECT to_regclass($1) IS NOT NULL", &[&table]).await.with_context(|| format!("look up {role} {table}"))?.get(0);
    if !exists {
        bail!("{role} {table} does not exist in DATABASE_URL");
    }
    Ok(())
}

/// Rebuild the single-blob layout from the source's per-field blobs over
/// the window: stream the source rows, move each row's twelve field streams
/// into one pco_pack container, and COPY the containers out on a second
/// connection so the read and the write overlap.
async fn backfill(pool: &Pool, args: &Args) -> Result<()> {
    let (from, to) = args.window;
    let sink = pool.get().await.context("backfill writer client")?;
    let ddl = format!(
        "DROP TABLE IF EXISTS {0} CASCADE; \
         CREATE TABLE {0} (database_id bigint NOT NULL, granularity integer NOT NULL, start_at timestamptz NOT NULL, \
         end_at timestamptz NOT NULL, blob bytea NOT NULL) PARTITION BY LIST (granularity); \
         {1} \
         ALTER TABLE {0} ALTER COLUMN blob SET STORAGE EXTERNAL; \
         CREATE INDEX {0}_idx ON {0} USING btree (database_id, end_at, start_at, granularity)",
        args.blob_table,
        args.granularities
            .iter()
            .map(|granularity| format!("CREATE TABLE {}_{granularity} PARTITION OF {0} FOR VALUES IN ({granularity});", args.blob_table))
            .collect::<String>(),
    );
    sink.simple_query(&ddl).await.context("create blob table")?;

    // Bounded channel: eight chunk containers in flight bound memory while
    // the COPY writer drains at network speed.
    let (tx, mut rx) = mpsc::channel::<(i64, i32, DateTime<Utc>, DateTime<Utc>, Vec<u8>)>(8);
    let writer_pool = pool.clone();
    let blob_table = args.blob_table.clone();
    let writer = tokio::spawn(async move {
        let sink = writer_pool.get().await.context("backfill writer client")?;
        let statement =
            sink.prepare_cached(&format!("COPY {blob_table} (database_id, granularity, start_at, end_at, blob) FROM STDIN BINARY")).await?;
        let types = [Type::INT8, Type::INT4, Type::TIMESTAMPTZ, Type::TIMESTAMPTZ, Type::BYTEA];
        let mut writer = std::pin::pin!(tokio_postgres::binary_copy::BinaryCopyInWriter::new(sink.copy_in(&statement).await?, &types));
        while let Some((database_id, granularity, start_at, end_at, blob)) = rx.recv().await {
            writer.as_mut().write(&[&database_id, &granularity, &start_at, &end_at, &blob.as_slice()]).await?;
        }
        writer.finish().await?;
        Ok::<(), anyhow::Error>(())
    });

    let reader = pool.get().await.context("backfill reader client")?;
    let statement = reader
        .prepare_cached(&format!(
            "SELECT database_id, granularity, start_at, end_at, {} FROM {} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4",
            PAYLOAD_COLS.join(", "),
            args.table
        ))
        .await?;
    let params: [&(dyn ToSql + Sync); 4] = [&args.db_ids, &args.granularities, &from, &to];
    let mut rows = std::pin::pin!(reader.query_raw(&statement, params).await.context("start backfill read")?);
    let mut copied = 0usize;
    while let Some(row) = rows.try_next().await? {
        let chunk = backfill_chunk(&row)?;
        let blob = <QueryStat as PcoPack>::to_bytes(std::slice::from_ref(&chunk))?;
        tx.send((chunk.database_id, chunk.granularity, chunk.start_at, chunk.end_at, blob))
            .await
            .map_err(|_| anyhow::anyhow!("backfill writer died"))?;
        copied += 1;
        if copied % 100 == 0 {
            eprintln!("backfilled {copied} chunk rows into {}", args.blob_table);
        }
    }
    drop(tx);
    writer.await.context("backfill writer panicked")??;
    reader.execute(&format!("ANALYZE {}", args.blob_table), &[]).await?;
    println!("backfilled {copied} chunk rows into {}", args.blob_table);
    Ok(())
}

/// The backfill SELECT's row layout: the four plain columns, then the
/// twelve field blobs in table order. Bounds and index values are real
/// here (the measurement fold leaves them defaulted because it never reads
/// them).
fn backfill_chunk(row: &Row) -> Result<Chunk> {
    Ok(Chunk {
        database_id: row.get(0),
        granularity: row.get(1),
        start_at: row.get(2),
        end_at: row.get(3),
        collected_at: ByteBuf::from(row.get::<_, &[u8]>(4)),
        fingerprint: ByteBuf::from(row.get::<_, &[u8]>(5)),
        postgres_role_id: ByteBuf::from(row.get::<_, &[u8]>(6)),
        toplevel: ByteBuf::from(row.get::<_, &[u8]>(7)),
        calls: ByteBuf::from(row.get::<_, &[u8]>(8)),
        rows: ByteBuf::from(row.get::<_, &[u8]>(9)),
        total_time: ByteBuf::from(row.get::<_, &[u8]>(10)),
        io_time: ByteBuf::from(row.get::<_, &[u8]>(11)),
        shared_blks_hit: ByteBuf::from(row.get::<_, &[u8]>(12)),
        shared_blks_read: ByteBuf::from(row.get::<_, &[u8]>(13)),
        temp_blks_read: ByteBuf::from(row.get::<_, &[u8]>(14)),
        temp_blks_written: ByteBuf::from(row.get::<_, &[u8]>(15)),
    })
}

/// Run every strategy against every shape on this runtime, returning cells
/// in report order: three strategy rows, each with one cell per shape. The
/// pool must have been built on the runtime this is awaited on;
/// `parallel_decode` switches the simple strategy to its worker handoff and
/// belongs to the multi-thread runtime only.
async fn measure_loads(pool: &Pool, args: &Args, shapes: &[Shape], parallel_decode: bool) -> Result<Vec<Vec<String>>> {
    let db = pool.get().await.context("checkout postgres client")?;
    let mut strategies: Vec<Vec<String>> = Vec::new();
    for pass in 0..3 {
        let mut row = Vec::new();
        for shape in shapes {
            let payload = shape.payload();
            let pair = match (pass, parallel_decode) {
                (0, false) => measure!(&db, args, simple_query(&db, shape, &payload, args).await),
                (0, true) => measure!(&db, args, simple_query_parallel_decode(&db, shape, &payload, args, args.threads).await),
                (1, _) => measure!(&db, args, tuples_parallel(pool, shape, &payload, args).await),
                (2, _) => measure!(&db, args, tuples_pipelined(&db, shape, &payload, args).await),
                _ => unreachable!("passes are 0..3"),
            };
            row.push(cell(pair));
        }
        eprintln!("measured {} against {} shapes", ["simple", "streamed", "pipelined"][pass], shapes.len());
        strategies.push(row);
    }
    Ok(strategies)
}

/// The single request production issues today: `database_id = ANY($1)
/// AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4`, every
/// requested database in one array, served by the production index. Rows
/// fold inline on this task as the server delivers them.
async fn simple_query(db: &deadpool_postgres::Client, shape: &Shape, payload: &str, args: &Args) -> Result<usize> {
    let statement = db
        .prepare_cached(&format!(
            "SELECT database_id, {payload} FROM {} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4",
            shape.table
        ))
        .await?;
    let (from, to) = args.window;
    let params: [&(dyn ToSql + Sync); 4] = [&args.db_ids, &args.granularities, &from, &to];
    let mut rows = std::pin::pin!(db.query_raw(&statement, params).await?);
    let mut count = 0usize;
    let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
    let mut consumer = pco_pack::fold!(QueryStat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
        count += 1;
        let slot = agg.entry((database_id, fingerprint)).or_default();
        slot.0 += calls;
        slot.1 += total_time;
    });
    while let Some(row) = rows.try_next().await? {
        fold_row(shape.kind, &row, &mut consumer)?;
    }
    black_box((count, agg.len()));
    Ok(count)
}

/// The `= ANY` stream with a decode handoff: each owned row goes to one of
/// `workers` tasks over a bounded channel, so pco decoding spreads over the
/// runtime's threads even though one socket delivers the rows in order.
async fn simple_query_parallel_decode(db: &deadpool_postgres::Client, shape: &Shape, payload: &str, args: &Args, workers: usize) -> Result<usize> {
    let statement = db
        .prepare_cached(&format!(
            "SELECT database_id, {payload} FROM {} WHERE database_id = ANY($1) AND granularity = ANY($2) AND end_at >= $3 AND start_at <= $4",
            shape.table
        ))
        .await?;
    let (from, to) = args.window;
    let params: [&(dyn ToSql + Sync); 4] = [&args.db_ids, &args.granularities, &from, &to];
    let mut rows = std::pin::pin!(db.query_raw(&statement, params).await?);

    let (sinks, mut streams): (Vec<mpsc::Sender<Row>>, Vec<_>) = (0..workers).map(|_| mpsc::channel::<Row>(8)).unzip();
    let kind = shape.kind;
    let mut handles = Vec::new();
    for mut rx in streams.drain(..) {
        handles.push(tokio::spawn(async move {
            let mut count = 0usize;
            let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
            let mut consumer = pco_pack::fold!(QueryStat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                count += 1;
                let slot = agg.entry((database_id, fingerprint)).or_default();
                slot.0 += calls;
                slot.1 += total_time;
            });
            while let Some(row) = rx.recv().await {
                fold_row(kind, &row, &mut consumer)?;
            }
            black_box((count, agg.len()));
            Ok::<usize, anyhow::Error>(count)
        }));
    }

    let mut next = 0;
    while let Some(row) = rows.try_next().await? {
        sinks[next].send(row).await.map_err(|_| anyhow::anyhow!("decode worker died"))?;
        next = (next + 1) % workers;
    }
    drop(sinks);

    let mut total = 0usize;
    for result in join_all(handles).await {
        total += result.context("decode worker panicked")??;
    }
    Ok(total)
}

/// One streaming request per (database_id, granularity) index tuple, each
/// with the production equality and range predicates, spread over pooled
/// connections so server-side toast fetches keep running while the client
/// folds earlier chunks.
async fn tuples_parallel(pool: &Pool, shape: &Shape, payload: &str, args: &Args) -> Result<usize> {
    let kind = shape.kind;
    let table = shape.table.clone();
    let payload = payload.to_string();
    let (from, to) = args.window;

    let mut handles = Vec::new();
    for &database_id in &args.db_ids {
        for &granularity in &args.granularities {
            let client = pool.get().await.context("checkout for tuple")?;
            let (table, payload) = (table.clone(), payload.clone());
            handles.push(tokio::spawn(async move {
                let statement = client
                    .prepare_cached(&format!(
                        "SELECT database_id, {payload} FROM {table} WHERE database_id = $1 AND granularity = $2 AND end_at >= $3 AND start_at <= $4"
                    ))
                    .await?;
                let params: [&(dyn ToSql + Sync); 4] = [&database_id, &granularity, &from, &to];
                let mut rows = std::pin::pin!(client.query_raw(&statement, params).await?);
                let mut count = 0usize;
                let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
                let mut consumer = pco_pack::fold!(QueryStat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                    count += 1;
                    let slot = agg.entry((database_id, fingerprint)).or_default();
                    slot.0 += calls;
                    slot.1 += total_time;
                });
                while let Some(row) = rows.try_next().await? {
                    fold_row(kind, &row, &mut consumer)?;
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
/// connection: tokio-postgres pipelines the requests onto one socket and
/// delivers responses in order. Submission and row consumption must
/// interleave: the connection task dispatches responses to the head query,
/// so submitting everything before consuming anything stalls the socket the
/// moment the head query's rows wait for a consumer. `join_all` interleaves
/// the futures on this task, which is the documented pipelining pattern
/// without the `Send` requirement (client handles are not `Clone`).
async fn tuples_pipelined(db: &deadpool_postgres::Client, shape: &Shape, payload: &str, args: &Args) -> Result<usize> {
    let kind = shape.kind;
    let statement = db
        .prepare_cached(&format!(
            "SELECT database_id, {payload} FROM {} WHERE database_id = $1 AND granularity = $2 AND end_at >= $3 AND start_at <= $4",
            shape.table
        ))
        .await?;
    let tuples: Vec<(i64, i32)> = args.db_ids.iter().flat_map(|&id| args.granularities.iter().map(move |&g| (id, g))).collect();
    let (from, to) = args.window;

    let requests = tuples.iter().map(|&(database_id, granularity)| {
        let (client, statement) = (db, statement.clone());
        async move {
            let params: [&(dyn ToSql + Sync); 4] = [&database_id, &granularity, &from, &to];
            let mut rows = std::pin::pin!(client.query_raw(&statement, params).await?);
            let mut count = 0usize;
            let mut agg: HashMap<(i64, i64), (i64, f64)> = HashMap::new();
            let mut consumer = pco_pack::fold!(QueryStat, |database_id: i64, fingerprint: i64, calls: i64, total_time: f64| {
                count += 1;
                let slot = agg.entry((database_id, fingerprint)).or_default();
                slot.0 += calls;
                slot.1 += total_time;
            });
            while let Some(row) = rows.try_next().await? {
                fold_row(kind, &row, &mut consumer)?;
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

/// Feed one result row's payload through the fold engine. The measurement
/// SELECT is `SELECT database_id, {payload}`, so both layouts keep
/// `database_id` live for the fold to key by: the per-field layout reads its
/// twelve blobs positionally, the single blob layout unpacks one container.
fn fold_row<C: pco_pack::Consumer<QueryStat>>(kind: ShapeKind, row: &Row, consumer: &mut C) -> Result<()> {
    let query = json!({});
    match kind {
        ShapeKind::Blob => {
            let chunks = <QueryStat as PcoPack>::from_bytes(row.get::<_, &[u8]>(1))?;
            <QueryStat as PcoPack>::fold_value(&chunks, &query, &[], consumer)?;
        }
        ShapeKind::Cols => {
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
            <QueryStat as PcoPack>::fold_value(std::slice::from_ref(&chunk), &query, &[], consumer)?;
        }
    }
    Ok(())
}

/// Evict every unpinned buffer, so each iteration starts with the shared
/// cache as empty as Postgres allows. `pg_buffercache_evict` is superuser-
/// only and needs the extension, neither of which RDS grants; that is why
/// eviction is opt-in here.
async fn cold_evict(db: &deadpool_postgres::Client) -> Result<()> {
    db.query_one(
        "SELECT count(*) FROM pg_buffercache WHERE relfilenode IS NOT NULL AND pinning_backends = 0 AND pg_buffercache_evict(bufferid)",
        &[],
    )
    .await
    .context("cold eviction needs the pg_buffercache extension (CREATE EXTENSION pg_buffercache) and superuser")?;
    Ok(())
}

/// The markdown-tables crate takes its header from a const fn, but which
/// shape columns exist is a command-line decision, so this prints the same
/// pipe-table shape by hand: header row, width-matched separator, and escaped
/// values.
fn markdown_table(header: &[String], rows: &[Vec<String>]) -> String {
    let escape = |values: &[String]| values.iter().map(|value| value.replace('|', "\\|")).collect::<Vec<_>>();
    let names = escape(header);
    let cells: Vec<Vec<String>> = rows.iter().map(|row| escape(row)).collect();
    let widths: Vec<usize> = names
        .iter()
        .enumerate()
        .map(|(column, name)| {
            std::iter::once(name.len()).chain(cells.iter().map(|row| row.get(column).map_or(0, String::len))).max().expect("column exists")
        })
        .collect();
    let line = |values: &[String]| {
        let mut out = String::from("|");
        for (value, width) in std::iter::zip(values, &widths) {
            out.push(' ');
            out.push_str(value);
            out.push_str(&" ".repeat(width - value.len()));
            out.push(' ');
            out.push('|');
        }
        out
    };
    let mut lines = vec![line(&names), line(&widths.iter().map(|width| "-".repeat(*width)).collect::<Vec<_>>())];
    lines.extend(cells.iter().map(|row| line(row)));
    lines.join("\n") + "\n"
}
