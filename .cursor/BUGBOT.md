# Anonymiser — Repo-Specific BugBot Rules

> Org-wide rules are enforced via Cursor Team Rules.

## Architecture

- Rust CLI tool that reads PostgreSQL dump files and anonymises them based on a strategy JSON file. Source is in `src/` with `parsers/` (SQL parsing), `fixers/` (data transformation), and top-level modules for file handling and compression.
- The tool processes SQL dumps line-by-line for memory efficiency. Changes to parsing or fixing logic must handle large files without loading everything into memory.

## Data Safety

- This tool processes database backups that may contain PII. Any changes to anonymisation logic must ensure data is properly scrubbed — incomplete anonymisation is a data privacy risk.
- Strategy files define how each database column is anonymised. The `generate-strategies` command creates blank strategy files from live databases. Review generated strategies carefully before use.

## Rust Conventions

- Edition 2021 Rust. Uses `structopt` for CLI argument parsing, `serde`/`serde_json` for JSON handling, `fake` for generating anonymised data.
- `mimalloc` is used as the global allocator for performance. Do not switch allocators without benchmarking.

## Testing & Releases

- Test files for anonymisation are in `test_files/`. Tests are in-source (`#[cfg(test)]` modules) and integration-style.
- Releases are published to GitHub Releases. The `update_version` script handles version bumping. Version in `Cargo.toml` is only used for dev builds.
