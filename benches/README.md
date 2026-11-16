# Benchmarks

These benchmarks use the `query_stats` table from the pganalyze staging environment (which doesn't contain any customer data). This data is collected from [pg_stat_statements](https://www.postgresql.org/docs/current/pgstatstatements.html) every minute and then sent to pganalyze every 10 minutes.

For more detailed benchmarks, see pco_pack's [benches/README.md](https://github.com/pganalyze/pco_pack/blob/main/benches/README.md).

Size is listed in megabytes, and times are listed in seconds.

## `bucket_size.rs`

Compacting the data from 10 minute buckets to 24 hour buckets improves the compression ratio and read/write time.

The ideal bucket size will depend on your workload. A larger bucket results in better compression, but means more unwanted data has to be loaded and discarded at read time.

|                                    | Size | Write time | Read time | Average bucket size |
| ---------------------------------- | ---- | ---------- | --------- | ------------------- |
| 1 day bucket (pco)                 | 217  | 9.0        | 1.0       | 28,433              |
| 10 minute bucket (pco)             | 321  | 18.8       | 2.0       | 214                 |
| 10 minute bucket (Postgres arrays) | 485  |            |           | 214                 |

## `comparison.rs`

Now with the optimized data model, this benchmark compares the performance of using pco, pco_store, or Postgres array types.

|                 | Size | Write time | Read time | Compression method |
| --------------- | ---- | ---------- | --------- | ------------------ |
| pco             | 107  | 8.3        | 0.8       | pco                |
| pco_store       | 120  | 9.9        | 1.1       | pco                |
| Postgres arrays | 207  | 53.5       | 5.2       | Postgres pglz      |

## `synthetic.rs`

Unlike the previous benchmarks, this generates synthetic data and measures the time and peak memory usage of each operation. The final fold section runs one decode-bound `fold!` projection through `for_each` and then through `for_each_parallel` at 2, 4, and 8 worker threads, asserting every variant sums to the same values; the peak rises with the worker count because the pipeline window holds about `workers + 1` decoded chunks while decoding.

| Operation                        | Time    | Peak memory |
| -------------------------------- | ------- | ----------- |
| store                            | 1.0 s   | 2304 MB     |
| load                             | 612 ms  | 1767 MB     |
| reduce                           | 617 ms  | 34 MB       |
| filter                           | 2 ms    | 9 MB        |
| fold (serial `for_each`)         | 145 ms  | 6 MB        |
| fold (`for_each_parallel`, 2)    | 80 ms   | 17 MB       |
| fold (`for_each_parallel`, 4)    | 76 ms   | 28 MB       |
| fold (`for_each_parallel`, 8)    | 78 ms   | 49 MB       |

The fold speedup plateaus past 2 workers because row delivery, the container copy, and the fold walk all stay serial on the fold thread; only decode leaves, and decode is about two thirds of the serial fold.

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
