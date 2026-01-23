# pco_store [![crates.io][crates_badge]][crates_url]

[crates_badge]: https://img.shields.io/crates/v/pco_store.svg
[crates_url]: https://crates.io/crates/pco_store

This crate uses [pco](https://github.com/pcodec/pcodec) to get the best possible compression ratio for numeric data, extending it with an easy to use API so you don't have to convert between row and columnar data structures yourself.

Postgres is currently required, though contributions are welcome to support other storage models.

To see the generated code, look in [tests/expand](tests/expand) or run `cargo expand --test tests`.

## Supported data types

- pco supports `u16`, `u32`, `u64`, `i16`, `i32`, `i64`, `f16`, `f32`, `f64`
- pco_store adds support for `chrono::DateTime`, `std::time::SystemTime`, `bool`

## Performance

Numeric compression algorithms take advantage of the mathematic relationships between a series of numbers to compress them to a higher degree than binary compression can. Of the numeric compression algorithms available in Rust, pco achieves both the best compression ratio and the best round-trip read and write time.

Compared to Postgres array data types, pco_store improves the compression ratio by 2x and improves read and write time by 5x in the included [benchmarks](benches). Better compression ratios can be expected with larger datasets.

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

#[pco_store::store(timestamp = collected_at, group_by = [database_id, granularity], float_round = 2)]
pub struct QueryStat {
    pub database_id: i64,
    /// Number of seconds captured in the query stat. 60 = 1 minute source data, 3600 = 1 hour aggregation
    pub granularity: i32,
    pub collected_at: SystemTime,
    pub fingerprint: i64,
    pub calls: i64,
}
```

And a matching Postgres table:

```sql
CREATE TABLE query_stats (
    database_id bigint NOT NULL,
    granularity int NOT NULL,
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    collected_at bytea STORAGE EXTERNAL NOT NULL,
    fingerprint bytea STORAGE EXTERNAL NOT NULL,
    calls bytea STORAGE EXTERNAL NOT NULL
) PARTITION BY LIST (granularity);

CREATE TABLE query_stats_1min PARTITION OF query_stats FOR VALUES IN (60);
CREATE TABLE query_stats_1hour PARTITION OF query_stats FOR VALUES IN (3600);

CREATE INDEX ON query_stats USING btree (database_id, end_at, start_at, granularity);
```

The stats can be:
- written with `store`
- read with `load` + `decompress`
- rewritten for better compression with `delete` + `store_grouped`

```rs
async fn example() -> anyhow::Result<()> {
    let database_id = 1;
    let granularity = 60;
    let start = SystemTime::UNIX_EPOCH;
    let end = SystemTime::now();
    let db = &DB_POOL.get().await?;

    // Write
    let default = QueryStat { database_id, granularity, collected_at: end, fingerprint: 1, calls: 1 };
    let stats = vec![QueryStat { collected_at: end - Duration::from_secs(120), ..default }];
    CompressedQueryStats::store(db, stats).await?;
    let stats = vec![QueryStat { collected_at: end - Duration::from_secs(60), ..default }];
    CompressedQueryStats::store(db, stats).await?;

    // Read
    let mut calls = 0;
    let filter = Filter::new(&[database_id], &[granularity], start..=end);
    for group in CompressedQueryStats::load(db, filter.clone()).await? {
        for stat in group.decompress()? {
            calls += stat.calls;
        }
    }
    assert_eq!(calls, 2);

    // Delete and re-group to improve compression ratio. This example compacts data into a single row per day.
    // The ideal group size will depend on the size and volume of your data.
    assert_eq!(2, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    transaction!(db, {
        let mut stats = Vec::new();
        for group in CompressedQueryStats::delete(db, filter.clone()).await? {
            stats.extend(group.decompress()?);
        }
        assert_eq!(0, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
        CompressedQueryStats::store_grouped(db, stats, |stat| {
            let collected_at: chrono::DateTime<chrono::Utc> = stat.collected_at.into();
            collected_at.duration_trunc(chrono::Duration::days(1)).ok()
        })
        .await?;
    });
    assert_eq!(1, db.query_one("SELECT count(*) FROM query_stats", &[]).await?.get::<_, i64>(0));
    let group = CompressedQueryStats::load(db, filter).await?.remove(0);
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

## Filtering

pco_store generates a `Filter` struct to specify read-time filters. Required fields from `group_by` and `fingerprint` will be filtered in SQL before the data is decompressed, but other fields can be filtered after decompression but before the data is returned to the caller as an optimization to avoid pointless allocations.

Timestamps are accepted as an inclusive range (with precision automatically truncated to microseconds), and all other fields are accepted as an array to check for inclusion in that array.

### Creating a filter

`Filter::new()` is a shorthand to set the required fields from `group_by` and `timestamp`. After it's created, additional filters can be set as fields on the struct. Struct literal syntax can also be used: `Filter { field: [1], ..Filter::default() }`

Filters can be deserialized using serde. Non-timestamp fields can be passed as a single JSON value which is automatically wrapped in an array.

Timestamps support multiple formats:
- `["ts1", "ts2"]`: an array with two timestamp strings becomes an inclusive range `ts1..=ts2`
- `["ts1"]`: an array with a single timestamp string becomes an inclusive range `ts1..=ts1`
- `"ts1"`: a single timestamp string becomes an inclusive range `ts1..=ts1`

### Filter convenience functions

- `range_bounds` returns the time range lower and upper bounds
- `range_duration` returns the duration of the filter's time range
- `range_shift` mutably shifts the time range's start and end by a certain amount, e.g. to filter for "today, 7 days ago"

## Contributions are welcome to

- support decompression of only the fields requested at runtime
- support other storage models (filesystem, S3, etc)
- support compression for other data types (text, enums, etc)
- add a stream/generator API to avoid allocating Vecs when loading data
- [add `copy_in` support to deadpool_postgres and tokio_postgres `GenericClient`](https://github.com/deadpool-rs/deadpool/issues/397)

## Other crates

These crates also implement numeric compression:

|                | Maintained? | Data type support        |
| -------------- | ----------- | ------------------------ |
| [stream-vbyte] | No          | Missing `i64` and floats |
| [bitpacking]   | No          | Missing `i64` and floats |
| [tsz-compress] | No          | Missing floats           |

[stream-vbyte]: https://crates.io/crates/stream-vbyte
[bitpacking]: https://crates.io/crates/bitpacking
[tsz-compress]: https://crates.io/crates/tsz-compress
