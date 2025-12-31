# Contributing to `pco_store`

## Environment Setup

To get started working on `pco_store`, you need to be able to run the tests.

To do this you need to:

1. Have running postgres instance.
2. Define the `DATABASE_URL` environment variable.
3. Install `cargo-expand` (`cargo install cargo-expand`).

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
