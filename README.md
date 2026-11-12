# pco_store [![crates.io][crates_badge]][crates_url]

[crates_badge]: https://img.shields.io/crates/v/pco_store.svg
[crates_url]: https://crates.io/crates/pco_store

Storage for columnar-compressed data using [pco_pack](https://github.com/pganalyze/pco_pack). One `Backend` trait, one impl per storage adapter: `store`, `load`, and `delete` on compressed chunks, efficiently filtered at query time.

Under the hood, numeric fields use [Pcodec](https://github.com/pcodec/pcodec) compression, while other types are compressed with MessagePack + zstd. See pco_pack's README for full details on [supported data types](https://github.com/pganalyze/pco_pack#supported-types) and [schema evolution](https://github.com/pganalyze/pco_pack#schema-evolution).

## Backends and the macros

`#[pco_store(...)]` and `#[derive(PcoStore)]` together implement the `Backend` trait for the backends your crate enables, so a store holds one runtime-picked adapter and call sites never change across rungs. The attribute declares every model setting once, the shared `name` and the pco_pack knobs, and forwards the knobs to pco_pack directly, so `#[derive(PcoPack)]` and `#[pco_pack(...)]` never appear in your code. pco_store wraps pco_pack entirely: it re-exports the crate (at `pco_store::pco_pack`), the `fold!` macro, and every model's typed filter as `pco_store::Filter<M>`, so your Cargo.toml and imports need only pco_store. Backends are crate features, not per-model attributes: `fs` is on by default and `postgres` brings the Postgres stack. The pair injects no wrapper type and no names into your scope. `#[derive(PcoStore)]` comes first and `#[pco_store(...)]` goes below it, the same shape pco_pack uses: the derive reads the settings for the `Backend` impls, then the attribute macro rewrites the struct to attach pco_pack:

```rs
use chrono::{DateTime, Utc};
use pco_store::PcoStore;
use pco_store::pco_store;

#[derive(Clone, PcoStore)]
#[pco_store(name = query_stats, timestamp = collected_at, index = [database_id])]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: DateTime<Utc>,
    pub calls: i64,
}
```

- `name` prefixes every storage object for the model: the Postgres table, the filesystem file names, future object keys. It defaults to the struct name in snake_case with a plural `s` (`QueryStat` becomes `query_stats`).
- `fs` persists each chunk as one file in a durable directory. The file name carries the table, the sanitized `index` key, and the timestamp bounds, so `load` narrows by directory listing before reading any bytes. Local storage is the only backend this build ships by default.
- `postgres` re-homes the upstream COPY, SELECT, and DELETE plumbing. It compiles only under the `postgres` feature, so local-only builds never carry the Postgres stack.
- An `object` backend is reserved for the offload rung and lands later.

The `PcoStore` trait gives every model its `store`, `load`, `delete`, and `for_each`: `store` takes rows and groups them into chunks, `load` returns storage-level `Chunk`s, `delete` returns whole chunks with empty filters so a re-store loses nothing, and `for_each` streams a whole load through one consumer (a `fold!` projection, if you want the speed) without materializing rows. The derive implements it, so `QueryStat::store(db, rows)` reads like data access without putting inherent methods on your struct; the trait shares the derive's name, and one `use pco_store::PcoStore` brings both into scope. A `Chunk` derefs to the pco_pack chunk it holds, so index columns and the `start_at`/`end_at` bounds read directly, and `decompress` applies the filter and fields it was loaded under. Any `&Backend` answers the calls: a `Postgres` handle, a checked-out pooled client, an `Fs` directory, or an `Arc<dyn Backend<M>>` picked at runtime. The `Backend` trait is the adapter side of the same calls, and `capabilities()` reports what each adapter really guarantees, so the trait never flattens a cloud down to the atomicity of Postgres.

Each method borrows a pooled client per call, so sequences that must commit atomically live above pco_store.

## Filtering

The typed filter pco_pack generates, reached as `pco_store::Filter<QueryStat>`, has a `new` function accepting one argument per `index` field followed by the `timestamp` range, direct field accessors for simple data types, and `Index`/`IndexMut` for complex data types. `index` and `timestamp` filters are used in SQL WHERE clauses to narrow the chunks read from Postgres, and other filters are applied during decompression, so `decompress` only produces rows matching the filter.

See pco_pack's README for full details on [typed filters](https://github.com/pganalyze/pco_pack#typed-filters) and [filter types](https://github.com/pganalyze/pco_pack#filter-types).

## Loading a subset of fields

Read requests can specify which fields to decompress, skipping unnecessary work (pco_pack's [partial reads](https://github.com/pganalyze/pco_pack#partial-reads)):

- `&[]` loads all fields
- `["fingerprint", "calls"]` loads the requested fields, plus any fields referenced by the filter, which are always included

## Compaction

`delete` removes the matching chunks and returns them, so data can be rewritten into fewer, larger chunks to improve the compression ratio. Because a chunk can span a wider range than the filter, `decompress` on chunks returned by `delete` applies no filter and returns all of their rows, so re-storing the data loses nothing. The example below shows this workflow.

## Generated code

To see what pco_store generates, look at [pco_store_derive/tests/expand_derive](pco_store_derive/tests/expand_derive) or run `cargo expand --test tests`.

## Usage

Put every model setting on one `#[pco_store(...)]`, written below `#[derive(PcoStore)]`. `name` carries the shared storage prefix; `timestamp`, `index`, `float_round`, `time_round`, and `chunk_size` are forwarded to pco_pack automatically. The crate features decide which `Backend` impls exist. Each backend reports its atomicity honestly:

- `fs` (default) writes each chunk as one file in a durable directory.
- `postgres` runs COPY, SELECT, and DELETE against the named table. Compiles only under the `postgres` feature, so local-only builds never carry the Postgres stack.

On `#[pco_store(...)]`: `timestamp = field` marks the timestamp field for range-based chunk filtering (it must be `chrono::DateTime<Utc>`); `index = [fields]` stores those fields uncompressed for SQL-level filtering before decompression (supported types: `i64`, `i32`, `String`, `smol_str::SmolStr`, `uuid::Uuid`, `bool`); `float_round = N` rounds floats to `N` decimal places; `time_round` rounds timestamps; `chunk_size` bounds rows per chunk (default 262,144). See pco_pack's [macro settings](https://github.com/pganalyze/pco_pack#macro-settings).

### Example

Define a struct and create a matching Postgres table. The composite index covers `index` fields and the timestamp bounds so `load` and `delete` can narrow chunks in SQL:

```rs
use chrono::{DateTime, Utc};
use pco_store::PcoStore;
use pco_store::pco_store;

#[derive(Clone, Debug, PartialEq, PcoStore)]
#[pco_store(timestamp = collected_at, index = [database_id, granularity], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    /// Number of seconds captured in the query stat. 60 = 1 minute source data, 3600 = 1 hour aggregation
    pub granularity: i32,
    pub collected_at: DateTime<Utc>,
    pub fingerprint: i64,
    pub calls: i64,
}

pub type Filter = pco_store::Filter<QueryStat>;
```

```sql
CREATE TABLE query_stats (
    database_id bigint NOT NULL,
    granularity int NOT NULL,
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    collected_at bytea STORAGE EXTERNAL NOT NULL,
    fingerprint bytea STORAGE EXTERNAL NOT NULL,
    calls bytea STORAGE EXTERNAL NOT NULL
);

CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
```

Then read and write through the model's `PcoStore` functions. Any `&Backend` answers them, and here that is a checked-out pooled client:

```rs
async fn example() -> anyhow::Result<()> {
    let database_id = 1;
    let granularity = 60;
    let now = Utc::now();
    let db = &DB_POOL.get().await?;

    // Write rows; they group into whole chunks automatically.
    let default = QueryStat { database_id, granularity, collected_at: now, fingerprint: 1, calls: 1 };
    QueryStat::store(db, vec![QueryStat { collected_at: now - chrono::Duration::seconds(120), ..default }]).await?;
    QueryStat::store(db, vec![QueryStat { collected_at: now - chrono::Duration::seconds(60), ..default }]).await?;

    // Read. The generated WHERE narrows by index fields and timestamp bounds.
    let mut calls = 0;
    let filter = Filter::new([database_id], [granularity], DateTime::<Utc>::UNIX_EPOCH..=now);
    for chunk in QueryStat::load(db, &filter, &[]).await? {
        for stat in chunk.decompress()? {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Stream the same range through a consumer instead of building rows:
    // one fold spans every loaded chunk, and a fold! consumer decodes only
    // the columns it reads.
    let mut folded = 0;
    QueryStat::for_each(db, &filter, &[], |row: QueryStat| folded += row.calls).await?;
    assert_eq!(folded, calls);

    // Delete and re-store to compact the data into fewer rows, improving the compression ratio.
    // `delete` returns whole chunks with empty filters, so re-storing loses nothing.
    // The ideal row size will depend on the size and volume of your data.
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    let mut stats = Vec::new();
    for group in QueryStat::delete(db, &filter).await? {
        stats.extend(group.decompress()?);
    }
    assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    QueryStat::store(db, stats).await?;
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));

    let group = QueryStat::load(db, &filter, &[]).await?.remove(0);
    let stats = group.decompress()?;
    assert_eq!(stats[0].collected_at, now - chrono::Duration::seconds(120));
    assert_eq!(stats[1].collected_at, now - chrono::Duration::seconds(60));

    Ok(())
}

pub static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
    use std::str::FromStr;
    let url = std::env::var("DATABASE_URL").unwrap_or("postgresql://localhost:5432/postgres".to_string());
    let pg_config = tokio_postgres::Config::from_str(&url).unwrap();
    let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
    let mgr = deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    deadpool_postgres::Pool::builder(mgr).build().unwrap().into()
});
```
