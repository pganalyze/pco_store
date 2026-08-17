# pco_store [![crates.io][crates_badge]][crates_url]

[crates_badge]: https://img.shields.io/crates/v/pco_store.svg
[crates_url]: https://crates.io/crates/pco_store

Postgres-backed storage for columnar-compressed data using [pco_pack](https://github.com/pganalyze/pco_pack). Provides `store`, `load`, and `delete` operations on compressed chunks that are efficiently filtered at query time.

Under the hood, numeric fields use [Pcodec](https://github.com/pcodec/pcodec) compression, while other types are compressed with MessagePack + zstd. See pco_pack's README for full details on [supported data types](https://github.com/pganalyze/pco_pack#supported-types), [schema evolution](https://github.com/pganalyze/pco_pack#schema-evolution), and [filtering](https://github.com/pganalyze/pco_pack#filtering).

## Filtering

pco_pack's generated `Filter` struct has a `new` function accepting filters for `index` and `timestamp` fields, direct field accessors for simple data types, and `Index`+`IndexMut` for complex data types. `index` and `timestamp` filters are used in SQL WHERE clauses to narrow rows before decompression, and other filters are applied during decompression.

See pco_pack's README for full details on [typed filters](https://github.com/pganalyze/pco_pack#typed-filters) and [filter syntax](https://github.com/pganalyze/pco_pack#filter-syn).

## Loading a subset of fields

Read requests can specify which fields to decompress, skipping unnecessary work:

- `&[]` loads all fields
- `["fingerprint", "calls"]` loads the requested fields, plus the required `index` and `timestamp` fields

When optional filters are used, their referenced fields are automatically included.

## Generated code

To see what pco_store generates, look at [tests/expand](tests/expand) or run `cargo expand --test tests`.

## Usage

The `pco_store::store` macro generates a compressed storage wrapper type with `load`, `delete`, and `store` functions. It accepts these arguments:

- `timestamp = field` marks a timestamp field for range-based chunk filtering; adds `start_at`/`end_at` columns to the table
- `index = [fields]` groups rows by these fields and stores them uncompressed in Postgres for efficient SQL-level filtering before decompression
- `float_round = N` rounds float values to `N` decimal places to improve compression (values stored as i64 internally)
- `time_round = chrono::Duration::seconds(N)` rounds timestamps to the nearest multiple of the given duration (e.g. 10 seconds) for better compression
- `chunk_size = N` sets the chunk size (default 32,768) used for serialization
- `table_name` overrides the default table name (`StructName` becomes `struct_names`)

Relevant settings are automatically passed to pco_pack's derive macro.

### Example

Define a struct and create a matching Postgres table:

```rs
use std::time::{Duration, SystemTime};

#[pco_store::store(timestamp = collected_at, index = [database_id, granularity], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    /// Number of seconds captured in the query stat. 60 = 1 minute source data, 3600 = 1 hour aggregation
    pub granularity: i32,
    pub collected_at: SystemTime,
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
    let now = SystemTime::now();
    let db = &DB_POOL.get().await?;

    // Write
    let default = QueryStat { database_id, granularity, collected_at: now, fingerprint: 1, calls: 1 };
    CompressedQueryStats::store(db, vec![QueryStat { collected_at: now - Duration::from_secs(120), ..default }]).await?;
    CompressedQueryStats::store(db, vec![QueryStat { collected_at: now - Duration::from_secs(60), ..default }]).await?;

    // Read
    let mut calls = 0;
    let filter = Filter::new([database_id], [granularity], SystemTime::UNIX_EPOCH..=now);
    for chunk in CompressedQueryStats::load(db, filter, &[]).await? {
        for stat in chunk.decompress()? {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Delete and re-group to improve compression ratio.
    // This example compacts data into a single row per day.
    // The ideal group size will depend on the size and volume of your data.
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    transaction!(db, {
        let mut stats = Vec::new();
        for group in CompressedQueryStats::delete(db, filter.clone(), ()).await? {
            stats.extend(group.decompress()?);
        }
        assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
        CompressedQueryStats::store(db, stats).await?;
    });
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, filter, []).await?.remove(0);
    assert_eq!(group.start_at, end - Duration::from_secs(120));
    assert_eq!(group.end_at, end - Duration::from_secs(60));
    let stats = group.decompress()?;
    assert_eq!(stats[0].collected_at, end - Duration::from_secs(120));
    assert_eq!(stats[1].collected_at, end - Duration::from_secs(60));

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

#[macro_export]
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
pub use transaction;
```
