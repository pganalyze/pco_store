# Contributing to `pco_store`

## Environment Setup

To get started working on `pco_store`, you need to be able to run the tests.

To do this you need to:

1. Have running postgres instance.
2. Define the `DATABASE_URL` environment variable.
3. Install `cargo-expand` (`cargo install cargo-expand`) to view the generated code with `cargo expand --test tests`.

For spinning up a Postgres instance quickly, some variant of this command works well:

```
docker run --rm -it -p 5432:5432 -e "POSTGRES_PASSWORD=password" postgres:latest
```

If you run that exact command, the value you want for `DATABASE_URL` is `postgresql://postgres@localhost:5432/postgres`.

### Verifying your setup

You can test your setup by specifying `DATABASE_URL` and running `cargo test`:

```
DATABASE_URL=postgresql://postgres@localhost:5432/postgres cargo test
```

## Running tests

Simply run `cargo test`

The integration tests in `tests/` require the Postgres instance from above; the proc macro expansion tests do not. The expansion tests rewrite the generated code snapshots in `tests/expand` on every run, so if you change the macro, commit the updated snapshots alongside your changes (CI fails on uncommitted changes).

`pco_pack` is pulled from git (https://github.com/pganalyze/pco_pack). If you also develop `pco_pack` locally, you can point the dependency at your checkout with a `[patch]` section in `Cargo.toml`:

```toml
[patch."https://github.com/pganalyze/pco_pack"]
pco_pack = { path = "../pco_pack" }
```
