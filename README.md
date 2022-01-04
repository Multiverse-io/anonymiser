# Anonymiser [![CircleCI](https://circleci.com/gh/Multiverse-io/anonymiser/tree/main.svg?style=svg&circle-token=f96c8ae882765c9cb2219d4539a5bed696451202)](https://circleci.com/gh/Multiverse-io/anonymiser/tree/main)


## Creating releases

`./create_releases` will create 3 releases:
- anonymiser-aarch64-apple-darwin (for m1 macs)
- anonymiser-x86_64-unknown-linux-gnu (for debian)
- anonymiser-x86_64-unknown-linux-musl (for alpine)
in the root of the project
if you just want to build for your current arch you can run `cargo build --release` and it will create an anonymiser binary in `targets/release/`


## Data Types

The following data types are supported

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
- FakeCompanyName - Random Company Name from [faker](https://github.com/cksac/fake-rs) *
- FakeEmail - Random email address from [faker](https://github.com/cksac/fake-rs) *
- FakeFirstNam - Random first name from [faker](https://github.com/cksac/fake-rs)
- FakeFullAddress - Random address made up of segments from [faker](https://github.com/cksac/fake-rs)
- FakeFullName - Random first plus last name from [faker](https://github.com/cksac/fake-rs)
- FakeIPv4 - Random IPV4 address from [faker](https://github.com/cksac/fake-rs)
- FakeLastName - Random last name from [faker](https://github.com/cksac/fake-rs)
- FakeNationalIdentityNumber - Random National Insurance number from list of dummy numbers
- FakePhoneNumber - Random phone number (looks at existing numbers country code, supports GB + US)
- FakePostCode - Truncates postcode to the first 3 chars e.g. NW5
- FakeState - Random US state from [faker](https://github.com/cksac/fake-rs)
- FakeStreetAddress - Random building number + street name from [faker](https://github.com/cksac/fake-rs)
- FakeUsername - Random username from [faker](https://github.com/cksac/fake-rs) *
- FakeUUID - Random UUIDv4
- Fixed - Returns a fixed value (requires a `value` arg with the value to use)
- Identity - Does not transform the original value
- ObfuscateDay - Takes a date and sets the day to the first of the month e.g. 12-12-2000 becomes 01-12-2000
- Scramble - Replaces text with random alphanumeric characters of the same length. Preserves spaces so word count is unchanged


Some transformers support option args. e.g. Fixed

```
  "transformer": {
    "name": "Fixed",
    "args": {
      "value": "new-value"
    }
  }
```

All instances of this field with be replaced with `new-value`

Transformers with a * at the end of the line support the arg `unique` which will append an incrementing number to the random data to guarantee no duplicates will occur.


