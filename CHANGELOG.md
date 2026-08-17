# Changelog

## 0.3.0

- **Breaking**: Use [pco_pack](https://github.com/pganalyze/pco_pack) for serialization, replacing the custom pco-based implementation
  - Rename `group_by` to `index`
  - Remove `store_grouped`; `store` groups rows into chunks automatically, so compact data with `delete` + `store`
  - `timestamp` fields must be `chrono::DateTime<Utc>`; `SystemTime` is no longer supported
  - Add `time_round` and `chunk_size` arguments, passed through to pco_pack
- **Breaking**: Prefix generated struct with `Compressed` for clarity
- **Breaking**: Skip time range filter in `decompress` when used by `delete`
  - This prevents data loss when compacting data into a single row to improve compression
- Add documentation to the generated code

## 0.2.0

- Add support for boolean fields

## 0.1.0

- Initial release
