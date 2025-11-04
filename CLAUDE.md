# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Anonymiser is a Rust CLI tool that reads PostgreSQL SQL backups (created with `pg_dump`) and anonymises them based on a strategy file. It processes SQL dumps line-by-line, transforming data according to configured strategies while preserving database structure.

## Essential Commands

### Building and Testing
```bash
# Build the project
cargo build

# Run all tests (requires PostgreSQL running at localhost:5432)
./build_and_test

# Run tests manually
cargo test

# Format code
cargo fmt

# Check code with clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### Running the Anonymiser
```bash
# Generate a strategy file from a database
anonymiser generate-strategies --db-url postgresql://postgres:postgres@localhost/DB_NAME

# Anonymise a SQL dump
anonymiser anonymise -i clear_text_dump.sql -o anonymised.sql -s strategy.json

# Check strategy file against database
anonymiser check-strategies --db-url postgresql://... --strategy-file strategy.json

# Fix strategy file errors
anonymiser fix-strategies --db-url postgresql://... --strategy-file strategy.json

# Export strategies to CSV
anonymiser to-csv --strategy-file strategy.json --output-file output.csv

# Helper functions for debugging
anonymiser anonymise-email --email "user@example.com" --salt "optional-salt"
anonymiser anonymise-id --id "user123" --transformer "FakeUUID" --args '{"deterministic": "true"}'
```

### Running Individual Tests
```bash
# Run a specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run tests with output
cargo test -- --nocapture
```

## Architecture

### High-Level Flow
1. **Strategy Loading** (`src/parsers/strategy_file.rs`): Reads strategy.json and validates configurations
2. **Database Schema Parsing** (`src/parsers/db_schema.rs`): When checking/generating strategies, connects to PostgreSQL to fetch table schemas
3. **Line-by-Line Processing** (`src/file_reader.rs`): Reads SQL dump line by line to minimize memory usage
4. **Row Parsing** (`src/parsers/row_parser.rs`): Determines row type (CREATE TABLE, COPY, data, etc.)
5. **Transformation** (`src/parsers/transformer.rs`): Applies configured transformers to data values
6. **Output Writing** (`src/file_reader.rs`): Writes transformed SQL to output file (optionally compressed)

### Key Modules

**Strategy Management** (`src/parsers/`):
- `strategy_structs.rs`: Core types (DataCategory, TransformerType, ColumnInfo, etc.)
- `strategies.rs`: Strategies struct that maps table names to column strategies
- `strategy_file.rs`: Reading/writing strategy.json files
- `custom_classifications.rs`: Support for custom data categories beyond built-in ones

**SQL Parsing** (`src/parsers/`):
- `row_parser.rs`: Main entry point for parsing each line
- `copy_row.rs`: Handles PostgreSQL COPY statements that introduce table data
- `data_row.rs`: Parses and transforms actual data rows
- `create_row.rs`: Parses CREATE TABLE statements to extract column types

**Transformation** (`src/parsers/`):
- `transformer.rs`: All transformer implementations (FakeEmail, Scramble, etc.)
- `sanitiser.rs`: Escapes special characters for SQL output
- `rng.rs`: Random number generation
- `types.rs`: PostgreSQL type system representation

**Validation** (`src/fixers/`):
- `db_mismatch.rs`: Detects differences between strategy file and database schema
- `validation.rs`: Validates strategy file consistency (PII not using Identity, no Error transformers, etc.)
- `fixer.rs`: Automatically fixes certain strategy file errors

**State Management**:
- `src/parsers/state.rs`: Tracks current table being processed, column types, etc. during line-by-line parsing

### Important Patterns

**Deterministic Transformations**:
Transformers marked with † in README support deterministic generation using `get_faker_rng()` which creates a seeded RNG from:
- Input value
- Optional ID column value (for entity-level consistency)
- Optional global salt (for run-level consistency)

This ensures the same input always generates the same output, critical for maintaining referential integrity.

**Global Salt**:
Strategy files can include a salt configuration as the first item:
```json
[
  {"salt": "your-global-salt-here"},
  {"table_name": "public.users", ...}
]
```

**Column Type Tracking**:
The system parses CREATE TABLE statements to track PostgreSQL column types, enabling type-aware transformations (e.g., array handling, date formatting).

**Memory Efficiency**:
The tool processes SQL dumps line-by-line without loading entire files into memory, making it suitable for multi-GB dumps. Uses `mimalloc` as global allocator for performance.

**Error Transformer Pattern**:
Columns with `"transformer": {"name": "Error"}` will cause anonymisation to fail. This forces explicit decisions about how to handle each column.

## Testing

Tests are embedded in source files using `#[cfg(test)]` modules. Integration tests in `src/anonymiser.rs` require:
- PostgreSQL running at `localhost:5432` with user `postgres` password `postgres`
- Permission to create/drop test databases

Test data lives in `test_files/` directory.

## Key Dependencies

- `fake`: Generates fake data (names, emails, addresses)
- `postgres`: Database connection for schema inspection
- `regex`: SQL pattern matching
- `structopt`: CLI argument parsing
- `serde_json`: Strategy file parsing
- `sha2/sha256`: Deterministic hashing
- `zstd/flate2`: Output compression
- `mimalloc`: Fast memory allocator

## Transformer Args

When adding transformer arguments:
- Add fields to transformer struct in `strategy_structs.rs`
- Parse in `transformer.rs` transform function
- Update README.md transformer documentation
- Consider whether deterministic mode is appropriate

## Custom Classifications

Users can define custom data categories beyond the built-in ones (General, Pii, PotentialPii, CommercialySensitive, Security, Unknown). The `--classifications-file` flag accepts a JSON file listing valid custom categories. The system validates that all custom categories in strategy files are defined.
