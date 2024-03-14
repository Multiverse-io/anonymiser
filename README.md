# Anonymiser [![CircleCI](https://dl.circleci.com/status-badge/img/gh/Multiverse-io/anonymiser/tree/main.svg?style=svg)](https://circleci.com/gh/Multiverse-io/anonymiser/tree/main)

A command line tool to read a sql backup (created with pg_dump) and anonymise it based on a strategy file

## Installing
There are a few options:

1. The binary can be downloded from [the releases page](https://github.com/Multiverse-io/anonymiser/releases).
2. An [asdf](https://github.com/asdf-vm/asdf) plugin is available at [Multiverse-io/asdf-anonymiser](https://github.com/Multiverse-io/asdf-anonymiser).
3. This repository is a [Nix flake](https://nix.dev/concepts/flakes) and can be used as input to your own flakes.

## Running
1. Ensure you have a strategy.json file (you can generate a blank one using `anonymiser generate-strategies --db-url postgres://postgres:postgres@localhost/DB_NAME`
2. Choose a category / transformer for the fields (details below)
3. Create a clear text backup with `pg_dump -x --no-owner > clear_text_dump.sql`
4. Run the anonymiser with `anonymiser anonymise -i clear_text_dump.sql -o anonymised.sql -s strategy.json`

For further command line options you can use `--help`

## Development

If you have Nix installed you can run `nix develop` inside the repository to open a subshell with the requisite development tools made available to you.
If you also have direnv installed you can run `direnv allow` to automatically open the subshell upon entering the repository directory.

Otherwise you just need to ensure a Rust toolchain is available, as provided by [rustup](https://www.rust-lang.org/tools/install) for example.

## Creating releases

1. Checkout the lastest main branch on your machine
2. Create a git tag with the new version number `git tag v1.2.3`
3. Push the tag `git push origin v1.2.3`
4. Wait for CircleCI to create a draft release
5. Review the draft release and publish


## Data Categories

The following data categories are supported

- CommerciallySensitive - Client names, addresses, anything that we might want to obfuscate for commercial reasons
- General - Normal data, not sensitive
- PotentialPii - Pretty much anything free text! Shouldn't contain PII but we can't guarantee that a user hasn't put their bank details and mothers maiden name in
- Pii - Personally Identifiable Information (e.g. phone number, email, name etc)
- Security - Related to the security of the system (e.g password hashes, magic links etc)
- Unknown - Unclassified, If any fields have this anonymisation will fail until it is replaced with a valid type


## Transformers
- EmptyJson - Literally `{}`
- Error - Not set. If any fields have this anonymisation will fail until it is replaced with a valid transformer
- FakeBase16String - Random Base16 string
- FakeBase32String - Random Base32 string
- FakeCity - Random city from [faker](https://github.com/cksac/fake-rs)
- FakeCompanyName * - Random Company Name from [faker](https://github.com/cksac/fake-rs)
- FakeEmail * - Random email address from [faker](https://github.com/cksac/fake-rs)
- FakeEmailOrPhone * - Either a random phone number OR a random email depending on whether the existing data starts with a `+` and doesn't contain an `@` symbol or not!
- Random email address from [faker](https://github.com/cksac/fake-rs)
- FakeFirstName - Random first name from [faker](https://github.com/cksac/fake-rs)
- FakeFullAddress - Random address made up of segments from [faker](https://github.com/cksac/fake-rs)
- FakeFullName - Random first plus last name from [faker](https://github.com/cksac/fake-rs)
- FakeIPv4 - Random IPV4 address from [faker](https://github.com/cksac/fake-rs)
- FakeLastName - Random last name from [faker](https://github.com/cksac/fake-rs)
- FakeNationalIdentityNumber - Random National Insurance number from list of dummy numbers
- FakePhoneNumber - Random phone number (looks at existing numbers country code, supports GB + US)
- FakePostCode - Truncates postcode to the first 3 chars e.g. NW5
- FakeState - Random US state from [faker](https://github.com/cksac/fake-rs)
- FakeStreetAddress - Random building number + street name from [faker](https://github.com/cksac/fake-rs)
- FakeUsername * - Random username from [faker](https://github.com/cksac/fake-rs)
- FakeUUID - Random UUIDv4
- Fixed - Returns a fixed value (requires a `value` arg with the value to use)
- Identity - Does not transform the original value
- ObfuscateDay - Takes a date and sets the day to the first of the month e.g. 12-12-2000 becomes 01-12-2000
- Scramble - Replaces text with random alphanumeric characters of the same length. Preserves spaces so word count is unchanged


Some transformers support option args. e.g. Fixed

```
  {
    "data_category": "Pii",
    "description": "",
    "name": "naughty_field",
    "transformer": {
      "name": "Fixed",
      "args": {
        "value": "new-value"
      }
    }
  },
```

All instances of this field with be replaced with `new-value`

Transformers with a * support the arg `unique` which will append an incrementing number to the random data to guarantee no duplicates will occur e.g.

```
  {
    "data_category": "Pii",
    "description": "user email address",
    "name": "email",
    "transformer": {
      "name": "FakeEmail",
      "args": {
        "unique": "true"
      }
    }
  },
```
