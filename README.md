# pco_store [![crates.io][crates_badge]][crates_url]

[crates_badge]: https://img.shields.io/crates/v/pco_store.svg
[crates_url]: https://crates.io/crates/pco_store

Postgres-backed storage for columnar-compressed data using [pco_pack](https://github.com/pganalyze/pco_pack). Provides `store`, `load`, and `delete` operations on compressed chunks that are efficiently filtered at query time.

Under the hood, numeric fields use [Pcodec](https://github.com/pcodec/pcodec) compression, while other types are compressed with MessagePack + zstd. See pco_pack's README for full details on [supported data types](https://github.com/pganalyze/pco_pack#supported-types) and [schema evolution](https://github.com/pganalyze/pco_pack#schema-evolution).

## Filtering

pco_pack's generated `Filter` struct has a `new` function accepting one argument per `index` field followed by the `timestamp` range, direct field accessors for simple data types, and `Index`/`IndexMut` for complex data types. `index` and `timestamp` filters are used in SQL WHERE clauses to narrow the chunks read from Postgres, and other filters are applied during decompression, so `decompress` only produces rows matching the filter.

See pco_pack's README for full details on [typed filters](https://github.com/pganalyze/pco_pack#typed-filters) and [filter types](https://github.com/pganalyze/pco_pack#filter-types).

## Loading a subset of fields

Read requests can specify which fields to decompress, skipping unnecessary work (pco_pack's [partial reads](https://github.com/pganalyze/pco_pack#partial-reads)):

- `&[]` loads all fields
- `["fingerprint", "calls"]` loads the requested fields, plus any fields referenced by the filter, which are always included

## Compaction

`delete` removes the matching chunks and returns them, so data can be rewritten into fewer, larger chunks to improve the compression ratio. Because a chunk can span a wider range than the filter, `decompress` on chunks returned by `delete` applies no filter and returns all of their rows, so re-storing the data loses nothing. The example below shows this workflow.

## Generated code

To see what pco_store generates, look at [tests/expand](tests/expand) or run `cargo expand --test tests`.

## Usage

The `pco_store::store` macro generates a compressed storage wrapper type (`CompressedStructName`) with `load`, `delete`, and `store` functions. It accepts these arguments:

- `timestamp = field` marks a timestamp field for range-based chunk filtering; adds `start_at`/`end_at` columns to the table. The field must be `chrono::DateTime<Utc>`
- `index = [fields]` groups rows by these fields and stores them uncompressed in Postgres for efficient SQL-level filtering before decompression. Supported types: `i64`, `i32`, `String`, `smol_str::SmolStr`, `uuid::Uuid`, `bool`
- `float_round = N` rounds float values to `N` decimal places to improve compression (values stored as i64 internally)
- `time_round = chrono::Duration::seconds(N)` rounds timestamps to the nearest multiple of the given duration (e.g. 10 seconds) for better compression
- `chunk_size = N` sets the chunk size (default 32,768) used for serialization
- `table_name` overrides the default table name (`StructName` becomes `struct_names`)

All settings are automatically passed to pco_pack's derive macro (see its [macro settings](https://github.com/pganalyze/pco_pack#macro-settings)).

### Example

Define a struct and create a matching Postgres table. The composite index covers `index` fields and the timestamp bounds so `load` and `delete` can narrow chunks in SQL:

```rs
use chrono::{DateTime, Utc};

#[pco_store::store(timestamp = collected_at, index = [database_id, granularity], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    /// Number of seconds captured in the query stat. 60 = 1 minute source data, 3600 = 1 hour aggregation
    pub granularity: i32,
    pub collected_at: DateTime<Utc>,
    pub fingerprint: i64,
    pub calls: i64,
}
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

Then store and load data:

```rs
async fn example() -> anyhow::Result<()> {
    let database_id = 1;
    let granularity = 60;
    let now = Utc::now();
    let db = &DB_POOL.get().await?;

    // Write
    let default = QueryStat { database_id, granularity, collected_at: now, fingerprint: 1, calls: 1 };
    CompressedQueryStats::store(db, vec![QueryStat { collected_at: now - chrono::Duration::seconds(120), ..default }]).await?;
    CompressedQueryStats::store(db, vec![QueryStat { collected_at: now - chrono::Duration::seconds(60), ..default }]).await?;

    // Read
    let mut calls = 0;
    let filter = Filter::new([database_id], [granularity], DateTime::<Utc>::UNIX_EPOCH..=now);
    for chunk in CompressedQueryStats::load(db, filter, &[]).await? {
        for stat in chunk.decompress()? {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Delete and re-store to compact the data into fewer rows, improving the compression ratio.
    // The ideal row size will depend on the size and volume of your data.
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    transaction!(db, {
        let mut stats = Vec::new();
        for group in CompressedQueryStats::delete(db, filter.clone(), &[]).await? {
            stats.extend(group.decompress()?);
        }
        assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
        CompressedQueryStats::store(db, stats).await?;
    });
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, filter, &[]).await?.remove(0);
    assert_eq!(group.start_at, now - chrono::Duration::seconds(120));
    assert_eq!(group.end_at, now - chrono::Duration::seconds(60));
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

macro_rules! transaction {
    ($db: ident, $block: expr) => {
        $db.execute("BEGIN", &[]).await?;
        let result: anyhow::Result<()> = (|| async {
            $block
            Ok(())
        })().await;
        match result {
            Ok(result) => {
                $db.execute("COMMIT", &[]).await?;
                result
            }
            Err(err) => {
                $db.execute("ROLLBACK", &[]).await?;
                anyhow::bail!(err);
            }
        }
    }
}
```
