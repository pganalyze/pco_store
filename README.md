# pco_store [![crates.io][crates_badge]][crates_url]

[crates_badge]: https://img.shields.io/crates/v/pco_store.svg
[crates_url]: https://crates.io/crates/pco_store

This crate uses [pco](https://github.com/pcodec/pcodec) to get the best possible compression ratio for numeric data, extending it with an easy to use API so you don't have to convert between row and columnar data structures yourself.

Postgres is currently required, though contributions are welcome to support other storage models.

To see the generated code, look in [tests/expand](tests/expand) or run `cargo expand --test tests`.

## Supported data types

- pco supports `u16`, `u32`, `u64`, `i16`, `i32`, `i64`, `f16`, `f32`, `f64`
- pco_store additionally supports `SystemTime`, `bool`

## Performance

Numeric compression algorithms take advantage of the mathematic relationships between a series of numbers to compress them to a higher degree than binary compression is able to. Of the numeric compression algorithms available in Rust, pco achieves both the best compression ratio and the best round-trip write and read time.

Compared to native Postgres arrays using binary compression, pco_store improves the compression ratio by 2x and improves read and write time by 5x in the included [benchmarks](benches). Better compression ratios can be expected with larger datasets.

## Usage

The `pco_store::store` procedural macro accepts these arguments:

- `timestamp` accepts the field name for a timestamp in the struct. Timestamps are internally stored as an `i64` microsecond offset from the Unix epoch. This adds `start_at` and `end_at` timestamp columns to the resulting table. A composite index should cover `start_at` and `end_at`.
- `group_by` accepts one or more field names that are stored as uncompressed fields on the Postgres table that all other fields are grouped by. The fields are added as `load` filters, and `store` automatically groups the input data by them. A composite index should cover these fields.
- `float_round` sets the number of fractional decimal points to retain for float values. This helps improve the compression ratio when you don't need the full precision of the source data. Internally this stores the values as `i64`, with the fractional precision retained by multiplying by 10^N at write time, and then at read time casting to float and dividing by 10^N. Users should confirm that the generated integer values won't overflow past `i64::MAX`.
- `table_name` overrides the Postgres table name. By default it underscores and pluralizes the struct name, so `QueryStat` becomes `query_stats`.

Additional notes:

- Each group should contain ten thousand or more rows. If your data is collected in smaller buckets than that in real-time, you may want a background job that routinely deletes and re-inserts the data into a smaller number of rows to improve the compression ratio.
- As a tradeoff for the improved compression ratio and read/write time, any additional read-time filtering must be done in Rust instead of SQL. When evaluating this data model, you will want to compare the relative performance of this code in production versus the SQL queries it replaces.

## Example

With a Rust struct that groups timeseries stats into a single row per `database_id`, and only retains two fractional digits for float values:

```rs
use std::time::{Duration, SystemTime};

#[pco_store::store(timestamp = collected_at, group_by = [database_id], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    pub collected_at: SystemTime,
    pub fingerprint: i64,
    pub calls: i64,
    pub total_time: f64,
}
```

And a matching Postgres table:

```sql
CREATE TABLE query_stats (
    database_id bigint NOT NULL,
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    collected_at bytea STORAGE EXTERNAL NOT NULL,
    fingerprint bytea STORAGE EXTERNAL NOT NULL,
    calls bytea STORAGE EXTERNAL NOT NULL,
    total_time bytea STORAGE EXTERNAL NOT NULL
);
CREATE INDEX ON query_stats USING btree (database_id);
CREATE INDEX ON query_stats USING btree (end_at, start_at);
```

`STORAGE EXTERNAL` is set so that Postgres doesn't try to compress the already-compressed fields

This uses a `(end_at, start_at)` index because it's more selective than `(start_at, end_at)` for common use cases. For example when loading the last week of stats, the `end_at` filter is what's doing the work to filter out rows.
```sql
end_at >= now() - interval '7 days' AND start_at <= now()
```

The stats can be written with `store`, read with `load` + `decompress`, and deleted with `delete`:

```rs
async fn example() -> anyhow::Result<()> {
    let database_id = 1;
    let start = SystemTime::UNIX_EPOCH;
    let end = SystemTime::now();
    let db = &DB_POOL.get().await?;

    // Write
    let stats = vec![QueryStat { database_id, collected_at: end - Duration::from_secs(120), fingerprint: 1, calls: 1, total_time: 1.0 }];
    QueryStats::store(db, stats).await?;
    let stats = vec![QueryStat { database_id, collected_at: end - Duration::from_secs(60), fingerprint: 1, calls: 1, total_time: 1.0 }];
    QueryStats::store(db, stats).await?;

    // Read
    let mut calls = 0;
    for group in QueryStats::load(db, &[database_id], start, end).await? {
        for stat in group.decompress()? {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Delete and re-group to improve compression ratio
    //
    // Note: you'll want to choose the time range passed to `delete` so it only groups, for example, stats
    // from the past day into a fewer number of rows. There's a balance to be reached between compression
    // ratio and not slowing down read queries with unwanted data from outside the requested time range.
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    transaction!(db, {
        let mut stats = Vec::new();
        for group in QueryStats::delete(db, &[database_id], start, end).await? {
            for stat in group.decompress()? {
                stats.push(stat);
            }
        }
        assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
        QueryStats::store(db, stats).await?;
    });
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    let group = QueryStats::load(db, &[database_id], start, end).await?.remove(0);
    assert_eq!(group.start_at, end - Duration::from_secs(120));
    assert_eq!(group.end_at, end - Duration::from_secs(60));
    let stats = group.decompress()?;
    assert_eq!(stats[0].collected_at, end - Duration::from_secs(120));
    assert_eq!(stats[1].collected_at, end - Duration::from_secs(60));

    Ok(())
}

use std::str::FromStr;

pub static DB_POOL: std::sync::LazyLock<std::sync::Arc<deadpool_postgres::Pool>> = std::sync::LazyLock::new(|| {
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

Additional examples can be found in [tests/tests.rs](tests/tests.rs).

## Contributions are welcome to

- support decompression of only the fields requested at runtime
- support other storage models (filesystem, S3, etc)
- support compression for other data types (text, enums, etc)
- add a stream/generator API to avoid allocating Vecs when loading data
- [add `copy_in` support to deadpool_postgres and tokio_postgres `GenericClient`](https://github.com/deadpool-rs/deadpool/issues/397)
