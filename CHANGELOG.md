# Changelog

## 0.3.0

- **Breaking**: Use [pco_pack](https://github.com/pganalyze/pco_pack) for serialization, replacing the custom pco-based implementation
  - Rename `group_by` to `index`
  - Remove `store_grouped`; `store` groups rows into chunks automatically, so compact data with `delete` + `store`
  - `timestamp` fields must be `chrono::DateTime<Utc>`; `SystemTime` is no longer supported
  - Add `time_round` and `chunk_size` arguments, passed through to pco_pack
- **Breaking**: Skip time range filter in `decompress` when used by `delete`
  - This prevents data loss when compacting data into a single row to improve compression
- **Breaking**: Rework storage around the `Backend` trait
  - Replace the `store` attribute macro and its generated `Compressed*` wrapper type with `#[derive(PcoStore)]` and a `#[pco_store(...)]` below it, the same shape pco_pack uses: the attribute declares the model's settings once, `name` (the prefix every backend shares) plus the pco_pack knobs, which it forwards to `pco_pack::PcoPack` automatically, and the derive emits the `Backend` impls
  - The derive also implements a `PcoStore` trait that answers `store`, `load`, and `delete` through any `&Backend`: a `Postgres` handle, a checked-out pooled client, an `Fs` directory, or `dyn Backend`; `store` takes rows and chunks them itself, and the model's inherent surface stays clean
  - pco_store wraps pco_pack entirely: generated code names it through `pco_store::pco_pack`, the crate re-exports `fold!`, and the typed filter is reachable as `pco_store::Filter<M>`, so model code needs the pco_store dependency alone and never imports `PcoPack` (a `Model::Filter` path would sit ambiguously beside `PcoPack::Filter`, so the alias is crate-level)
  - `load` and `delete` return storage-level `Chunk`s that deref to the pco_pack chunk, so index columns and the `start_at`/`end_at` bounds read directly; `Chunk::for_each` folds one chunk through a `pco_pack::Consumer`, and `QueryStat::for_each(db, &filter, &fields, consumer)` streams a whole load into a single consumer, which is much cheaper with `fold!` projections than collecting rows
  - Backends are crate features rather than per-model attributes: `fs` by default, `postgres` opt-in
  - The Postgres stack compiles only behind the `postgres` feature, and the crate re-exports `tokio_postgres` and `deadpool_postgres` so generated adapters never name the stack themselves
- Add documentation to the generated code

## 0.2.0

- Add support for boolean fields

## 0.1.0

- Initial release
