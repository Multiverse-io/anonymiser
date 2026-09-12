# AGENTS.md

`anonymiser` is a Rust CLI that reads a `pg_dump` SQL backup and anonymises it according to a strategy file. See `README.md` for full usage, transformers, and data categories.

## Common commands

- Build: `cargo build`
- Lint: `cargo fmt -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test` (the `build_and_test` script wraps the full CI flow)
- Run: `cargo run -- <subcommand>` (e.g. `anonymise -i dump.sql -o out.sql -s strategy.json`)

## Cursor Cloud specific instructions

The update script already fetches Rust dependencies. The Rust toolchain is preinstalled (`rustc`/`cargo`).

Non-obvious notes for running the test suite:

- **Integration tests need Postgres.** Several tests in `src/anonymiser.rs` and `src/parsers/db_schema.rs` connect to `postgresql://postgres:postgres@localhost` (port 5432) and shell out to the `psql` client. Start Docker (`sudo nohup dockerd >/tmp/dockerd.log 2>&1 &`) and a Postgres container (`docker run -d --name anon-pg -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:13.4`) before `cargo test`. The `psql` client (`postgresql-client`) must also be on PATH.
- **Run tests single-threaded:** `cargo test -- --test-threads=1`. The file-IO tests share fixtures under `test_files/`, so the default parallel run can race and fail (e.g. an "incomplete frame" error on the compressed-file test). CI's Postgres-backed run avoids this; locally, single-threaded is the reliable invocation.
- The `build_and_test` script uses `RUSTC_BOOTSTRAP=1` to enable unstable test JSON output for `cargo2junit`; plain `cargo test` does not need it.
