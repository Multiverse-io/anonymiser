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

### Custom Classifications

You can use custom classifications by providing a local file path to a JSON file containing the classifications. The format of the file should be:

```json
{
  "classifications": [
    "InternalUseOnly",
    "Confidential",
    "Restricted",
    "Public"
  ]
}
```

To use custom classifications, add the `--classifications-file` flag to any command:

```
anonymiser anonymise -i clear_text_dump.sql -o anonymised.sql -s strategy.json --classifications-file ./path/to/classifications.json
```

In your strategy.json file, you can use custom classifications like this:

```json
{
  "data_category": "InternalUseOnly",
  "description": "Internal documentation",
  "name": "internal_notes",
  "transformer": {
    "name": "Scramble"
  }
}
```

**Behaviour with Invalid Custom Classifications:**

If your strategy file (`strategy.json`) references a custom classification in the `data_category` field for a column, but that classification is *not* defined in your custom classifications JSON file (or if no custom classifications file is provided), it will be treated as an invalid custom classification.

When `check-strategies` is run:
- A warning message will be displayed, highlighting the tables and columns that use these invalid custom classifications.
- The process will exit with an error code.

When `to-csv` is run:
- Columns with invalid custom classifications will still be included in the CSV output.
- The `data_category` field in the CSV will show the custom name (e.g., `Custom("MyInvalidType")`).
- An additional field/note `[INVALID CUSTOM CLASSIFICATION: MyInvalidType]` will be appended to the data category in the CSV to clearly mark it as invalid.

It is recommended to define all custom classifications you intend to use in the classifications file to ensure correct validation and behavior.

## Data transformation

Table data can be transformed in one of two ways,
1. Truncating the table
To use this option the table should be defined in the strategy file with the `truncate` key set to `true` and the `columns` key set to an empty array. e.g.
  ```
   {
    "table_name": "public.trunctable_table",
    "description": "",
    "truncate": true,
    "columns": []
  },
  ```

2. Transform the data in the table
Transforming table data requires a list of all table columns with a transformer defined for each and every column. (Note that for non PII or sensitive data, you can use the `Identity` transformer to not transform the data.

- EmptyJson - Literally `{}`
- Error - Not set. If any fields have this anonymisation will fail until it is replaced with a valid transformer
- FakeBase16String - Random Base16 string
- FakeBase32String - Random Base32 string
- FakeCity - Random city from [faker](https://github.com/cksac/fake-rs)
- FakeCompanyName * - Random Company Name from [faker](https://github.com/cksac/fake-rs)
- FakeEmail - Generates deterministic fake email addresses using a hash-based prefix. The output format is `<hash-prefix>-<random-email>` where the hash-prefix is derived from the original email and the random email is generated using [faker](https://github.com/cksac/fake-rs), ensuring consistent anonymisation across runs.
- FakeEmailOrPhone * - Either a random phone number OR a random email depending on whether the existing data starts with a `+` and doesn't contain an `@` symbol or not!
- FakeFirstName† - Random first name from [faker](https://github.com/cksac/fake-rs). Supports deterministic generation by setting `deterministic: true` and providing an `id_column` argument
- FakeFullAddress - Random address made up of segments from [faker](https://github.com/cksac/fake-rs)
- FakeFullName† - Random first plus last name from [faker](https://github.com/cksac/fake-rs). Supports deterministic generation by setting `deterministic: true` and providing an `id_column` argument
- FakeIPv4 - Random IPV4 address from [faker](https://github.com/cksac/fake-rs)
- FakeLastName†- Random last name from [faker](https://github.com/cksac/fake-rs). Supports deterministic generation by setting `deterministic: true` and providing an `id_column` argument
- FakeNationalIdentityNumber - Random National Insurance number from list of dummy numbers
- FakePhoneNumber - Random phone number (looks at existing numbers country code, supports GB + US)
- FakePostCode - Truncates postcode to the first 3 chars e.g. NW5
- FakeState - Random US state from [faker](https://github.com/cksac/fake-rs)
- FakeStreetAddress - Random building number + street name from [faker](https://github.com/cksac/fake-rs)
- FakeUsername * - Random username from [faker](https://github.com/cksac/fake-rs)
- FakeUUID† - Random UUIDv4, Supports deterministic generation by setting `deterministic: true`
- Fixed - Returns a fixed value (requires a `value` arg with the value to use)
- Identity - Does not transform the original value
- ObfuscateDay - Takes a date and sets the day to the first of the month e.g. 12-12-2000 becomes 01-12-2000
- ObfuscateDateTime - Takes a datetime and sets both the day to the first of the month and time to midnight (00:00:00) e.g. 2024-03-15 14:30:45 becomes 2024-03-01 00:00:00
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

Transformers with a † support deterministic generation by setting `deterministic: true` and providing required `id_column` argument (except for fakeUUID transformer, which doesn't require an id_column). This ensures the same input and ID always generate the same fake data.

Example of deterministic name generation:
```json
{
  "data_category": "Pii",
  "description": "user's first name",
  "name": "first_name",
  "transformer": {
    "name": "FakeFirstName",
    "args": {
      "deterministic": "true",
      "id_column": "user_account_id"
    }
  }
}
```

When using deterministic mode:
- The same input value and ID will always generate the same fake name
- The `id_column` must reference a valid ID column in the same table (e.g., "user_id", "user_account_id", "registrant_id" etc)
- If `deterministic` is true but the specified ID column is missing or invalid, the transformer will raise an error
- Different IDs will generate different names, even for the same input value

This is useful when you need consistent fake names across multiple database dumps or when maintaining referential integrity between tables.

## Global Salt

The anonymiser supports using a global salt for consistent hashing across different runs. To use this feature, add a salt configuration as the first item in your strategy.json file:

```json
[
  {
    "salt": "your-global-salt-here"
  },
  {
    "table_name": "public.users",
    "description": "",
    "columns": [
      // ... columns configuration ...
    ]
  }
]
```

The salt will be applied to all transformers that support salted hashing (marked with † in the transformer list). Different salt values will generate different outputs for the same input

## Helper Functions for Local Debugging

The anonymiser provides helper functions that can be used to get anonymised values for specific inputs. This is particularly useful for local debugging scenarios where you need to match production user IDs or emails to their anonymised counterparts.

### Anonymise Email

Get an anonymised email address for a given email:

```bash
anonymiser anonymise-email --email "user@example.com"
# Output: b4c9a289323b-cordia_iusto@hotmail.com

# With salt for different outputs
anonymiser anonymise-email --email "user@example.com" --salt "mysalt123"
# Output: b4c9a289323b-justus_ut@yahoo.com
```

### Anonymise ID

Get an anonymised ID for a given ID value:

```bash
# Using FakeUUID transformer with deterministic generation
anonymiser anonymise-id --id "user123" --transformer "FakeUUID" --args '{"deterministic": "true"}'
# Output: e4d6655a-4a20-1c4a-6740-d647ba4bf06e

# Using Scramble transformer
anonymiser anonymise-id --id "user456" --transformer "Scramble"
# Output: llzm895

# With salt for different outputs
anonymiser anonymise-id --id "user123" --transformer "FakeUUID" --args '{"deterministic": "true"}' --salt "mysalt123"
```



### Use Case: Local Debugging

When teams update their strategy.json files to anonymise user IDs and emails, they can use these helper functions to:

1. **Match Production Users**: Take a production user ID or email and get its anonymised equivalent
2. **Consistent Debugging**: Use the same salt and transformer settings as your strategy.json to ensure consistent results
3. **Quick Testing**: Test different transformer configurations before updating your strategy file

#### Example Workflow:

1. You have a production user with email `john.smith@company.com` and ID `12345`
2. Your strategy.json anonymises emails with `FakeEmail` and IDs with `FakeUUID` (deterministic)
3. Get the anonymised values:

```bash
# Get anonymised email
anonymiser anonymise-email --email "john.smith@company.com"

# Get anonymised ID (matching your strategy.json configuration)
anonymiser anonymise-id --id "12345" --transformer "FakeUUID" --args '{"deterministic": "true"}'
```

4. Use these anonymised values to identify the same user in your local anonymised database

This allows you to maintain the debugging workflow while keeping data properly anonymised and following security best practices.