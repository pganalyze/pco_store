# Benchmarks

These benchmarks use the `query_stats` table from the pganalyze staging environment (which doesn't contain any customer data). This data is collected from [pg_stat_statements](https://www.postgresql.org/docs/current/pgstatstatements.html) every minute and then sent to pganalyze every 10 minutes.

The first two benchmarks incrementally improve the compression ratio by changing the data model. Then the `comparison.rs` benchmark compares the resulting data model with different compression methods.

Size is listed in megabytes, and times are listed in seconds.

## Data model considerations

### `bucket_size.rs`

Compacting the data from 10 minute buckets to 24 hour buckets improves the compression ratio and read/write time.

The ideal bucket size will depend on your workload. A larger bucket results in better compression, but means more unwanted data has to be loaded and discarded at read time.

|                                    | Size | Write time | Read time | Average bucket size |
| ---------------------------------- | ---- | ---------- | --------- | ------------------- |
| 1 day bucket (pco)                 | 217  | 18.5       | 2.0       | 28,433              |
| 10 minute bucket (pco)             | 318  | 30.7       | 4.5       | 214                 |
| 10 minute bucket (Postgres arrays) | 485  |            |           | 214                 |

### `float.rs`

Rounding the `total_time` and `io_time` float values to varying levels of precision can significantly improve the compression ratio. Converting the floats into integers that are multiplied by 10^N at write time to preserve the desired fractional precision further improves the compression ratio.

Reducing the float precision to 2 decimal points reduces the size by 29% (217 MB -> 155 MB). Then using an integer representation further reduces the size by 31% (155 MB -> 107 MB). Combined, that's a 51% improvement.

|                                           | Size | Write time | Read time |
| ----------------------------------------- | ---- | ---------- | --------- |
| `bucket_size.rs` winner (as baseline)     | 217  | 18.5       | 2.0       |
| rounded to 0 decimals                     | 106  | 15.8       | 1.9       |
| rounded to 1 decimal                      | 132  | 16.6       | 1.8       |
| rounded to 2 decimals                     | 155  | 17.7       | 1.8       |
| multiplied by 1 and casted to integer     | 89   | 14.5       | 1.8       |
| multiplied by 10 and casted to integer    | 97   | 14.7       | 1.7       |
| multiplied by 100 and casted to integer   | 107  | 15.0       | 1.6       |

## Overall results

### `comparison.rs`

Now with the optimized data model, this benchmark compares the performance of using pco, pco_store, or Postgres array types.

|                 | Size | Write time | Read time | Compression method |
| --------------- | ---- | ---------- | --------- | ------------------ |
| pco             | 107  | 14.8       | 1.6       | pco                |
| pco_store       | 107  | 15.8       | 1.7       | pco                |
| Postgres arrays | 207  | 82.7       | 10.2      | Postgres pglz      |

## Others

### `chrono.rs`

The standard library `SystemTime` is being used depsite [chrono's](https://crates.io/crates/chrono) more feature-complete API because adding durations to a timestamp (in `decompress`) is noticeably slower when using chrono.

TODO: write this benchmark

## Disqualified crates

These crates don't support necessary data types, and in local tests they didn't outperform pco anyway:

- [stream-vbyte](https://crates.io/crates/stream-vbyte): doesn't support `i64` or floats
- [bitpacking](https://crates.io/crates/bitpacking): doesn't support `i64` or floats
- [tsz-compress](https://crates.io/crates/tsz-compress): doesn't support floats

# Setup

First install [git-lfs](https://docs.github.com/en/repositories/working-with-files/managing-large-files/installing-git-large-file-storage), then build the `query_stats` table from the compressed backup file:

```sh
pg_restore -c -d postgres benches/query_stats.db
```

Then run the benchmarks. The table sizes can be seen with this query:

```sql
ANALYZE;
SELECT name,
  pg_size_pretty(sum(total_bytes)) AS total,
  pg_size_pretty(sum(table_bytes)) AS table,
  pg_size_pretty(sum(toast_bytes)) AS toast,
  pg_size_pretty(sum(index_bytes)) AS index,
  sum(reltuples::int) AS rows
FROM (
  SELECT *, total_bytes - index_bytes - COALESCE(toast_bytes, 0) AS table_bytes
  FROM (
    SELECT relname AS name,
      pg_total_relation_size(c.oid) AS total_bytes,
      pg_indexes_size(c.oid) AS index_bytes,
      pg_total_relation_size(reltoastrelid) AS toast_bytes,
      reltuples
    FROM pg_class c
    LEFT JOIN pg_namespace n ON n.oid = relnamespace
    WHERE relkind = 'r' AND nspname = 'public'
  ) _
) _
GROUP BY name ORDER BY name;
```

### Internal: extract the query_stats table with the associated data model changes

```sql
ALTER TABLE postgres_roles DROP CONSTRAINT postgres_roles_pkey;
ALTER TABLE postgres_roles ADD COLUMN id_bigint bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY;
CREATE INDEX CONCURRENTLY ON postgres_roles USING btree (id);

CREATE TABLE query_stats (
    database_id bigint NOT NULL,
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    collected_at timestamptz[] NOT NULL,
    collected_secs bigint[] NOT NULL,
    fingerprint bigint[] NOT NULL,
    postgres_role_id bigint[] NOT NULL,
    calls bigint[] NOT NULL,
    rows bigint[] NOT NULL,
    total_time double precision[] NOT NULL,
    io_time double precision[] NOT NULL,
    shared_blks_hit bigint[] NOT NULL,
    shared_blks_read bigint[] NOT NULL
);
CREATE INDEX ON query_stats USING btree (database_id);
CREATE INDEX ON query_stats USING btree (end_at, start_at);

INSERT INTO query_stats
SELECT database_id,
    min_collected_at,
    (SELECT max(c) FROM unnest(collected_at) c),
    collected_at,
    collected_interval_secs,
    fingerprint,
    (SELECT array_agg(id_bigint) FROM unnest(postgres_role_id) p, postgres_roles WHERE id = p),
    calls,
    rows,
    total_time,
    (SELECT array_agg(r + w) FROM unnest(blk_read_time, blk_write_time) _(r, w)),
    shared_blks_hit,
    shared_blks_read
FROM query_stats_packed_35d;
```

And then run:
```sh
pg_dump -Z7 -Fc -O --table query_stats SOURCE_DB_NAME > benches/query_stats.db
```
