# Changelog

## 0.3.0

- **Breaking**: Prefix generated struct with `Compressed` for clarity
- **Breaking**: Skip time range filter in `decompress` when used by `delete`
  - This prevents data loss when compacting data into a single row to improve compression
- Add documentation to the generated code

## 0.2.0

- Add support for boolean fields

## 0.1.0

- Initial release
