use crate::parsers::national_insurance_number;
use crate::parsers::strategy_structs::{Transformer, TransformerType};
use crate::parsers::types::Type::Array;
use crate::parsers::types::Type::SingleValue;
use crate::parsers::types::*;
use base16;
use base32::Alphabet;
use chrono::{Datelike, NaiveDate};
use core::ops::Range;
use fake::faker::address::en::*;
use fake::faker::company::en::*;
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use log::trace;
use rand::SeedableRng;
use rand::{rngs::SmallRng, Rng};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

static UNIQUE_INTEGER: AtomicUsize = AtomicUsize::new(0);

fn get_unique() -> usize {
    UNIQUE_INTEGER.fetch_add(1, Ordering::SeqCst)
}

/// Creates a deterministic random number generator from input parameters.
///
/// # Arguments
///
/// * `value` - Base value to seed the RNG
/// * `id` - Optional identifier to ensure consistent generation for the same entity
/// * `salt` - Optional global salt to vary generation between runs
///
/// # Returns
///
/// A deterministic `SmallRng` that will produce the same sequence of values
/// for identical inputs.
///
/// # Examples
///
/// ```
/// // Basic usage with just a value
/// let rng1 = get_faker_rng("test", None, None);
///
/// // With an ID for entity-level consistency
/// let rng2 = get_faker_rng("test", Some("user_123"), None);
///
/// // With both ID and salt for run-level consistency
/// let rng3 = get_faker_rng("test", Some("user_123"), Some("global_salt_2024"));
/// ```
fn get_faker_rng(value: &str, id: Option<&str>, salt: Option<&str>) -> SmallRng {
    let mut hasher = Sha256::new();
    let combined = match (id, salt) {
        (Some(id), Some(salt)) => format!("{}{}{}", value, id, salt),
        (Some(id), None) => format!("{}{}", value, id),
        (None, Some(salt)) => format!("{}{}", value, salt),
        (None, None) => value.to_string(),
    };
    hasher.update(combined.as_bytes());
    let seed = u64::from_le_bytes(hasher.finalize()[..8].try_into().unwrap());
    SmallRng::seed_from_u64(seed)
}

pub fn transform<'line>(
    rng: &mut SmallRng,
    value: &'line str,
    column_type: &Type,
    transformer: &'line Transformer,
    table_name: &str,
    column_values: &[(String, String)],
    global_salt: Option<&str>,
) -> Cow<'line, str> {
    if ["\\N", "deleted"].contains(&value) {
        return Cow::from(value);
    }

    if transformer.name == TransformerType::Identity {
        return Cow::from(value);
    }

    if let Array {
        sub_type: underlying_type,
    } = column_type
    {
        return transform_array(
            rng,
            value,
            underlying_type,
            transformer,
            table_name,
            column_values,
            global_salt,
        );
    }

    let unique = get_unique();

    // Get the id value if specified in transformer args
    let id = transformer.args.as_ref().and_then(|args| {
        args.get("id_column").and_then(|id_column| {
            column_values
                .iter()
                .find(|(col, _)| col == id_column)
                .map(|(_, val)| val.as_str())
        })
    });

    //TODO error if inappropriate transformer for type is used e.g. scramble for json should give
    //nice error rather than making invalid sql

    match transformer.name {
        TransformerType::Error => {
            panic!("Error transform still in place for table: {}", table_name)
        }
        TransformerType::EmptyJson => Cow::from("{}"),
        TransformerType::FakeBase16String => Cow::from(fake_base16_string()),
        TransformerType::FakeBase32String => Cow::from(fake_base32_string()),
        TransformerType::FakeCity => Cow::from(CityName().fake::<String>()),
        TransformerType::FakeCompanyName => Cow::from(fake_company_name(
            value,
            &transformer.args,
            unique,
            global_salt,
        )),
        TransformerType::FakeEmail => Cow::from(fake_email(value, global_salt)),
        TransformerType::FakeEmailOrPhone => Cow::from(fake_email_or_phone(value, global_salt)),
        TransformerType::FakeFirstName => {
            Cow::from(fake_first_name(value, &transformer.args, id, global_salt))
        }
        TransformerType::FakeFullAddress => Cow::from(fake_full_address()),
        TransformerType::FakeFullName => {
            Cow::from(fake_full_name(value, &transformer.args, id, global_salt))
        }
        TransformerType::FakeIPv4 => Cow::from(IPv4().fake::<String>()),
        TransformerType::FakeLastName => {
            Cow::from(fake_last_name(value, &transformer.args, id, global_salt))
        }
        TransformerType::FakeNationalIdentityNumber => Cow::from(fake_national_identity_number()),
        TransformerType::FakePostCode => Cow::from(fake_postcode(value)),
        TransformerType::FakePhoneNumber => Cow::from(fake_phone_number(value)),
        TransformerType::FakeStreetAddress => Cow::from(fake_street_address()),
        TransformerType::FakeState => Cow::from(StateName().fake::<String>()),
        TransformerType::FakeUsername => Cow::from(fake_username(&transformer.args, unique)),
        TransformerType::Scramble => Cow::from(scramble(rng, value)),
        TransformerType::ScrambleBlank => Cow::from(scramble_blank(value)),
        TransformerType::ObfuscateDay => Cow::from(obfuscate_day(value, table_name)),
        TransformerType::Fixed => fixed(&transformer.args, table_name),
        TransformerType::Identity => Cow::from(value),
        TransformerType::FakeUUID => Cow::from(fake_uuid(value, &transformer.args, global_salt)),
    }
}

fn transform_array<'value>(
    rng: &mut SmallRng,
    value: &'value str,
    underlying_type: &SubType,
    transformer: &Transformer,
    table_name: &str,
    column_values: &[(String, String)],
    global_salt: Option<&str>,
) -> Cow<'value, str> {
    let quoted_types = [SubType::Character, SubType::Json];
    let requires_quotes = quoted_types.contains(underlying_type);

    let sub_type = SingleValue {
        sub_type: underlying_type.clone(),
    };

    let transformed_array = if requires_quotes {
        transform_quoted_array(
            rng,
            value,
            &sub_type,
            transformer,
            table_name,
            column_values,
            global_salt,
        )
    } else {
        let unsplit_array = &value[1..value.len() - 1];
        unsplit_array
            .split(", ")
            .map(|list_item| {
                transform(
                    rng,
                    list_item,
                    &sub_type,
                    transformer,
                    table_name,
                    column_values,
                    global_salt,
                )
            })
            .collect::<Vec<Cow<str>>>()
            .join(",")
    };
    Cow::from(format!("{{{}}}", transformed_array))
}

fn transform_quoted_array(
    rng: &mut SmallRng,
    value: &str,
    sub_type: &Type,
    transformer: &Transformer,
    table_name: &str,
    column_values: &[(String, String)],
    global_salt: Option<&str>,
) -> String {
    let mut inside_word = false;
    let mut word_is_quoted = false;
    let mut current_word: String = "".to_string();
    let mut word_acc: String = "".to_string();
    let mut last_char_seen: char = 'a';
    let last_char_index = value.len() - 1;
    for (i, c) in value.chars().enumerate() {
        trace!("-----------");
        trace!("current value is '{}'", c);
        if i == 0 {
            continue;
        } else if !inside_word && c == '"' {
            word_is_quoted = true;
            continue;
        } else if !inside_word && c == ',' {
            continue;
        } else if inside_word
            && ((word_is_quoted && c == '"' && last_char_seen != '\\')
                || (!word_is_quoted && c == ',')
                || (!word_is_quoted && c == '}'))
        {
            inside_word = false;
            word_is_quoted = false;
            let transformed = transform(
                rng,
                &current_word,
                sub_type,
                transformer,
                table_name,
                column_values,
                global_salt,
            );
            write!(word_acc, "\"{}\",", &transformed)
                .expect("Should be able to apppend to word_acc");
            current_word = "".to_string();
            trace!("its the end of a word");
        } else {
            inside_word = true;
            current_word.push(c);
        }

        last_char_seen = c;
        trace!(
            "current_word: '{}', inside_word: '{}', last_char_seen: '{}', index: '{}/{}'",
            current_word,
            inside_word,
            last_char_seen,
            i,
            last_char_index
        );
    }
    trace!("\noutput - {:?}", word_acc);
    //Remove the trailing comma from line: 145!
    word_acc.pop();
    word_acc
}

fn is_deterministic(args: &Option<HashMap<String, String>>) -> bool {
    args.as_ref()
        .and_then(|args| args.get("deterministic"))
        .is_some_and(|val| val == "true")
}

fn prepend_unique_if_present(
    new_value: String,
    args: &Option<HashMap<String, String>>,
    unique: usize,
) -> String {
    let unique_value = args
        .as_ref()
        .and_then(|a| a.get("unique"))
        .map_or_else(|| false, |u| u == "true");

    if unique_value {
        format!("{}-{}", unique, new_value)
    } else {
        new_value
    }
}

fn fake_base16_string() -> String {
    let random_bytes = SmallRng::from_rng(rand::thread_rng())
        .unwrap_or_else(|_| SmallRng::from_entropy())
        .gen::<[u8; 16]>();
    base16::encode_lower(&random_bytes)
}

fn fake_base32_string() -> String {
    let random_bytes = SmallRng::from_rng(rand::thread_rng())
        .unwrap_or_else(|_| SmallRng::from_entropy())
        .gen::<[u8; 16]>();
    base32::encode(Alphabet::Rfc4648 { padding: true }, &random_bytes)
}

fn fake_company_name(
    value: &str,
    args: &Option<HashMap<String, String>>,
    unique: usize,
    global_salt: Option<&str>,
) -> String {
    let mut seeded_rng = get_faker_rng(value, None, global_salt);
    let new_company_name = CompanyName().fake_with_rng::<String, _>(&mut seeded_rng);
    prepend_unique_if_present(new_company_name, args, unique)
}

fn fake_email(value: &str, global_salt: Option<&str>) -> String {
    let mut seeded_rng = get_faker_rng(value, None, global_salt);
    let new_email = FreeEmail().fake_with_rng::<String, _>(&mut seeded_rng);

    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash = hasher.finalize();
    let prefix = base16::encode_lower(&hash[..6]);

    format!("{}-{}", prefix, new_email)
}

fn fake_email_or_phone(current_value: &str, global_salt: Option<&str>) -> String {
    if current_value.starts_with('+') && !current_value.contains('@') {
        fake_phone_number(current_value)
    } else {
        fake_email(current_value, global_salt)
    }
}

fn fake_street_address() -> String {
    let building: String = BuildingNumber().fake();
    let street_name: String = StreetName().fake();
    format!("{} {}", building, street_name)
}

fn fake_full_address() -> String {
    let line_1 = fake_street_address();
    let city_name: String = CityName().fake();
    let state: String = StateName().fake();
    format!("{}, {}, {}", line_1, city_name, state)
}

fn fake_uuid(
    value: &str,
    args: &Option<HashMap<String, String>>,
    global_salt: Option<&str>,
) -> String {
    if !is_deterministic(args) {
        return Uuid::new_v4().to_string();
    }

    let mut seeded_rng = get_faker_rng(value, None, global_salt);

    Uuid::from_bytes(seeded_rng.gen()).to_string()
}

fn fake_first_name(
    value: &str,
    args: &Option<HashMap<String, String>>,
    id: Option<&str>,
    global_salt: Option<&str>,
) -> String {
    let deterministic = is_deterministic(args);
    let id_to_use = if deterministic { id } else { None };

    match id_to_use {
        Some(id) => {
            let mut seeded_rng = get_faker_rng(value, Some(id), global_salt);
            FirstName().fake_with_rng::<String, _>(&mut seeded_rng)
        }
        None => FirstName().fake::<String>(),
    }
}

fn fake_last_name(
    value: &str,
    args: &Option<HashMap<String, String>>,
    id: Option<&str>,
    global_salt: Option<&str>,
) -> String {
    let deterministic = is_deterministic(args);
    let id_to_use = if deterministic { id } else { None };

    match id_to_use {
        Some(id) => {
            let mut seeded_rng = get_faker_rng(value, Some(id), global_salt);
            LastName().fake_with_rng::<String, _>(&mut seeded_rng)
        }
        None => LastName().fake::<String>(),
    }
}

fn fake_full_name(
    value: &str,
    args: &Option<HashMap<String, String>>,
    id: Option<&str>,
    global_salt: Option<&str>,
) -> String {
    let deterministic = is_deterministic(args);
    let id_to_use = if deterministic { id } else { None };

    let first = fake_first_name(&format!("{}_first", value), args, id_to_use, global_salt);
    let last = fake_last_name(&format!("{}_last", value), args, id_to_use, global_salt);
    format!("{} {}", first, last)
}

fn fake_national_identity_number() -> String {
    //TODO currently this is free text so they can enter anything at all,
    //so im not bothering with us vs uk,
    //there dont seem to be any us social sec numbers in the DB currently
    national_insurance_number::random()
}

//https://www.ofcom.org.uk/phones-telecoms-and-internet/information-for-industry/numbering/numbers-for-drama
static UK_FAKE_MOBILE_RANGE: Range<i32> = 900000..960999;

fn fake_phone_number(current_value: &str) -> String {
    let mut rng =
        SmallRng::from_rng(rand::thread_rng()).unwrap_or_else(|_| SmallRng::from_entropy());
    if current_value.starts_with("+447") {
        let random = rng.gen_range(UK_FAKE_MOBILE_RANGE.clone());
        format!("+447700{0}", random)
    } else {
        let area_code = rng.gen_range(200..999);
        let rest = rng.gen_range(1000000..9999999);
        format!("+1{}{}", area_code, rest)
    }
}

fn fake_postcode(current_value: &str) -> String {
    //TODO not sure this is unicode safe...
    let mut truncated_value = current_value.to_string();
    truncated_value.truncate(3);
    truncated_value.to_string()
}

fn fake_username(args: &Option<HashMap<String, String>>, unique: usize) -> String {
    let username = Username().fake();
    prepend_unique_if_present(username, args, unique)
}

fn fixed<'a>(args: &'a Option<HashMap<String, String>>, table_name: &str) -> Cow<'a, str> {
    let value = args
        .as_ref()
        .and_then(|a| a.get("value"))
        .unwrap_or_else(|| {
            panic!(
                "'value' must be present in args for a fixed transformer in table: '{}'\ngot: '{:?}'",
                table_name, args,
            )
        });
    Cow::from(value)
}

fn obfuscate_day(value: &str, table_name: &str) -> String {
    match NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Ok(date) => {
            let new_date = date.with_day(1).unwrap();
            new_date.to_string()
        }
        Err(err) => {
            return value
                .strip_suffix(" BC")
                .and_then(|trimmed| {
                    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                        .ok()
                        .map(|re_parsed| format!("{} BC", re_parsed.with_day(1).unwrap()))
                })
                .unwrap_or_else(|| {
                    panic!(
                        "Invalid date found: \"{}\" in table: \"{}\". Error: \"{}\"",
                        value, table_name, err
                    )
                })
        }
    }
}

fn scramble(rng: &mut SmallRng, original_value: &str) -> String {
    let mut last_was_backslash = false;
    original_value
        .chars()
        .map(|c| {
            if last_was_backslash {
                last_was_backslash = false;
                c
            } else if c == '\\' {
                last_was_backslash = true;
                c
            } else if c == ' ' {
                c
            } else if c.is_ascii_digit() {
                rng.gen_range(b'0'..=b'9') as char
            } else {
                rng.gen_range(b'a'..=b'z') as char
            }
        })
        .collect::<String>()
}

fn scramble_blank(original_value: &str) -> String {
    let mut last_was_backslash = false;
    original_value
        .chars()
        .map(|c| {
            if last_was_backslash {
                last_was_backslash = false;
                c
            } else if c == '\\' {
                last_was_backslash = true;
                c
            } else if c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::rng;
    use regex::Regex;

    const TABLE_NAME: &str = "gert_lush_table";
    const EMPTY_COLUMNS: &[(String, String)] = &[];

    #[test]
    fn null_is_not_transformed() {
        let null = "\\N";
        let mut rng = rng::get();
        let new_null = transform(
            &mut rng,
            null,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_null, null);
    }

    #[test]
    fn deleted_is_not_transformed() {
        let deleted = "deleted";
        let mut rng = rng::get();
        let new_deleted = transform(
            &mut rng,
            deleted,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_deleted, deleted);
    }

    #[test]
    fn identity() {
        let first_name = "any first name";
        let mut rng = rng::get();
        let new_first_name = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Identity,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_first_name == first_name);
    }

    #[test]
    fn fake_base16_string() {
        let verification_key = "1702a4eddd53d6fa79ed4a677e64c002";
        let mut rng = rng::get();
        let new_verification_key = transform(
            &mut rng,
            verification_key,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeBase16String,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_verification_key != verification_key);
        assert_eq!(new_verification_key.len(), 32);
    }

    #[test]
    fn fake_base32_string() {
        let verification_key = "EMVXWNTUKRVAODPQ7KIBBQQTWY======";
        let mut rng = rng::get();
        let new_verification_key = transform(
            &mut rng,
            verification_key,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeBase32String,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_verification_key != verification_key);
        assert_eq!(new_verification_key.len(), 32);
    }

    #[test]
    fn fake_uuid_random() {
        let value = "some-value";
        let mut rng = rng::get();

        let transformer = Transformer {
            name: TransformerType::FakeUUID,
            args: None,
        };

        let uuid = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        // Verify it's a valid UUID
        assert!(
            Uuid::parse_str(&uuid).is_ok(),
            "Should generate a valid UUID"
        );

        // Verify it's different when called again
        let uuid2 = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(uuid, uuid2, "Random UUIDs should be different");
    }

    #[test]
    fn fake_uuid_deterministic() {
        let value1 = "some-value-1";
        let value2 = "some-value-2";
        let mut rng = rng::get();

        // Create transformer with deterministic args
        let transformer = Transformer {
            name: TransformerType::FakeUUID,
            args: Some(HashMap::from([(
                "deterministic".to_string(),
                "true".to_string(),
            )])),
        };

        // Same value should produce same UUID
        let uuid1_first_call = transform(
            &mut rng,
            value1,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        let uuid1_second_call = transform(
            &mut rng,
            value1,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_eq!(
            uuid1_first_call, uuid1_second_call,
            "Same input should produce same UUID"
        );

        // Different values should produce different UUIDs
        let uuid2 = transform(
            &mut rng,
            value2,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(
            uuid1_first_call, uuid2,
            "Different values should produce different UUIDs"
        );

        // Verify the outputs are valid UUIDs
        assert!(
            Uuid::parse_str(&uuid1_first_call).is_ok(),
            "Should generate a valid UUID"
        );
        assert!(
            Uuid::parse_str(&uuid2).is_ok(),
            "Should generate a valid UUID"
        );
    }

    #[test]
    fn fake_uuid_with_salt() {
        let value = "some-value";
        let mut rng = rng::get();

        // Create transformer with deterministic args
        let transformer = Transformer {
            name: TransformerType::FakeUUID,
            args: Some(HashMap::from([(
                "deterministic".to_string(),
                "true".to_string(),
            )])),
        };

        // Test with salt
        let uuid_with_salt = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("test_salt"),
        );

        // Test without salt
        let uuid_without_salt = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(
            uuid_with_salt, uuid_without_salt,
            "Same input with and without salt should produce different UUIDs"
        );

        // Test with different salt
        let uuid_with_different_salt = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("different_salt"),
        );

        assert_ne!(
            uuid_with_salt, uuid_with_different_salt,
            "Same input with different salts should produce different UUIDs"
        );

        // Verify the outputs are valid UUIDs
        assert!(
            Uuid::parse_str(&uuid_with_salt).is_ok(),
            "Should generate a valid UUID"
        );
        assert!(
            Uuid::parse_str(&uuid_without_salt).is_ok(),
            "Should generate a valid UUID"
        );
        assert!(
            Uuid::parse_str(&uuid_with_different_salt).is_ok(),
            "Should generate a valid UUID"
        );

        // Verify consistency with same salt
        let uuid_with_salt_repeat = transform(
            &mut rng,
            value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("test_salt"),
        );

        assert_eq!(
            uuid_with_salt, uuid_with_salt_repeat,
            "Same input with same salt should produce same UUID"
        );
    }

    #[test]
    fn fake_company_name() {
        let company_name = "any company name";
        let mut rng = rng::get();
        let transformer = Transformer {
            name: TransformerType::FakeCompanyName,
            args: None,
        };
        let new_company_name = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_company_name != company_name);

        let repeat_company_name = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(
            new_company_name, repeat_company_name,
            "Same input should produce same fake company name"
        );

        let different_company_name = transform(
            &mut rng,
            "different company name",
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_ne!(
            new_company_name, different_company_name,
            "Different inputs should produce different fake company names"
        );
    }

    #[test]
    fn fake_company_name_with_unique_arg() {
        let company_name = "any company name";
        let mut rng = rng::get();
        let transformer = &Transformer {
            name: TransformerType::FakeCompanyName,
            args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
        };
        let new_company_name = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_company_name != company_name);
        let re = Regex::new(r"^[0-9]+-.*").unwrap();
        assert!(
            re.is_match(&new_company_name),
            "Company name {:?} does not have the expected unique prefix format",
            new_company_name
        );
    }

    #[test]
    fn fake_company_name_with_salt() {
        let company_name = "Acme Inc";
        let mut rng = rng::get();

        // Create transformer
        let transformer = Transformer {
            name: TransformerType::FakeCompanyName,
            args: None,
        };

        // Test with salt
        let new_company_name_with_salt = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("test_salt"),
        );

        // Test without salt
        let new_company_name_without_salt = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(
            new_company_name_with_salt, new_company_name_without_salt,
            "Same input with and without salt should produce different fake company names"
        );

        // Test with different salt
        let new_company_name_with_different_salt = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("different_salt"),
        );

        assert_ne!(
            new_company_name_with_salt, new_company_name_with_different_salt,
            "Same input with different salts should produce different fake company names"
        );
    }

    #[test]
    fn fake_email_generates_consistent_output() {
        let email = "test@example.com";
        let mut rng = rng::get();
        let transformer = Transformer {
            name: TransformerType::FakeEmail,
            args: None,
        };

        let first_result = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        let second_result = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_eq!(
            first_result, second_result,
            "Same input should produce same fake email"
        );
    }

    #[test]
    fn fake_email_format_is_correct() {
        let email = "test@example.com";
        let mut rng = rng::get();
        let transformer = Transformer {
            name: TransformerType::FakeEmail,
            args: None,
        };

        let result = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        // Should match format: <6-char-hex>-<generated_email>
        let re = Regex::new(r"^[0-9a-f]{12}-[^@]+@[^@]+\.[^@]+$").unwrap();
        assert!(re.is_match(&result), "Email format incorrect: {}", result);
    }

    #[test]
    fn fake_email_different_inputs_produce_different_outputs() {
        let mut rng = rng::get();
        let transformer = Transformer {
            name: TransformerType::FakeEmail,
            args: None,
        };

        let email1 = "test1@example.com";
        let email2 = "test2@example.com";

        let result1 = transform(
            &mut rng,
            email1,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        let result2 = transform(
            &mut rng,
            email2,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(
            result1, result2,
            "Different inputs should produce different fake emails"
        );
    }

    #[test]
    fn fake_first_name_random() {
        let first_name = "any first name";
        let mut rng = rng::get();
        let new_first_name = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeFirstName,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(new_first_name, first_name);
    }

    #[test]
    fn fake_first_name_deterministic() {
        let first_name = "John Smith";
        let mut rng = rng::get();

        // Create transformer with deterministic args
        let transformer = Transformer {
            name: TransformerType::FakeFirstName,
            args: Some(HashMap::from([
                ("deterministic".to_string(), "true".to_string()),
                ("id_column".to_string(), "user_id".to_string()),
            ])),
        };

        let column_values = vec![("user_id".to_string(), "123".to_string())];

        let first_name_for_user1 = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        let repeat_first_name_for_user1 = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        assert_eq!(
            first_name_for_user1, repeat_first_name_for_user1,
            "Same input with same user_id should produce same fake name"
        );

        // Test with different user_id
        let column_values_user2 = vec![("user_id".to_string(), "456".to_string())];

        let first_name_for_user2 = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values_user2,
            None,
        );

        assert_ne!(
            first_name_for_user1, first_name_for_user2,
            "Same name with different user_ids should produce different fake names"
        );
    }

    #[test]
    fn fake_first_name_with_salt() {
        let first_name = "John";
        let mut rng = rng::get();

        // Create transformer with deterministic args
        let transformer = Transformer {
            name: TransformerType::FakeFirstName,
            args: Some(HashMap::from([
                ("deterministic".to_string(), "true".to_string()),
                ("id_column".to_string(), "user_id".to_string()),
            ])),
        };

        let column_values = vec![("user_id".to_string(), "123".to_string())];

        // Test with salt
        let first_name_with_salt = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            Some("test_salt"),
        );

        // Test without salt
        let first_name_without_salt = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        assert_ne!(
            first_name_with_salt, first_name_without_salt,
            "Same input with and without salt should produce different fake names"
        );

        // Test with different salt
        let first_name_with_different_salt = transform(
            &mut rng,
            first_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            Some("different_salt"),
        );

        assert_ne!(
            first_name_with_salt, first_name_with_different_salt,
            "Same input with different salts should produce different fake names"
        );
    }

    #[test]
    fn fake_full_name_random() {
        let full_name = "John Smith";
        let mut rng = rng::get();

        let transformer = Transformer {
            name: TransformerType::FakeFullName,
            args: None,
        };

        let new_full_name = transform(
            &mut rng,
            full_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(new_full_name, full_name);
    }

    #[test]
    fn fake_full_name_deterministic() {
        let full_name = "John Smith";
        let mut rng = rng::get();

        let transformer = Transformer {
            name: TransformerType::FakeFullName,
            args: Some(HashMap::from([
                ("deterministic".to_string(), "true".to_string()),
                ("id_column".to_string(), "user_id".to_string()),
            ])),
        };

        let column_values = vec![("user_id".to_string(), "123".to_string())];

        let full_name_for_user1 = transform(
            &mut rng,
            full_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        let repeat_full_name_for_user1 = transform(
            &mut rng,
            full_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        assert_eq!(
            full_name_for_user1, repeat_full_name_for_user1,
            "Same input with same user_id should produce same fake name"
        );

        // Test with different user_id
        let column_values_user2 = vec![("user_id".to_string(), "456".to_string())];

        let full_name_for_user2 = transform(
            &mut rng,
            full_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values_user2,
            None,
        );

        assert_ne!(
            full_name_for_user1, full_name_for_user2,
            "Same name with different user_ids should produce different fake names"
        );
    }

    #[test]
    fn fake_last_name_random() {
        let last_name = "any last name";
        let mut rng = rng::get();
        let new_last_name = transform(
            &mut rng,
            last_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeLastName,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(new_last_name, last_name);
    }

    #[test]
    fn fake_last_name_deterministic() {
        let last_name = "Smith";
        let mut rng = rng::get();

        let transformer = Transformer {
            name: TransformerType::FakeLastName,
            args: Some(HashMap::from([
                ("deterministic".to_string(), "true".to_string()),
                ("id_column".to_string(), "user_id".to_string()),
            ])),
        };

        let column_values = vec![("user_id".to_string(), "123".to_string())];

        let last_name_for_user1 = transform(
            &mut rng,
            last_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        let repeat_last_name_for_user1 = transform(
            &mut rng,
            last_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values,
            None,
        );

        assert_eq!(
            last_name_for_user1, repeat_last_name_for_user1,
            "Same input with same user_id should produce same fake name"
        );

        // Test with different user_id
        let column_values_user2 = vec![("user_id".to_string(), "456".to_string())];

        let last_name_for_user2 = transform(
            &mut rng,
            last_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            &column_values_user2,
            None,
        );

        assert_ne!(
            last_name_for_user1, last_name_for_user2,
            "Same name with different user_ids should produce different fake names"
        );
    }

    #[test]
    fn fake_full_address() {
        let street_address = "any street_address";
        let mut rng = rng::get();
        let new_street_address = transform(
            &mut rng,
            street_address,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeFullAddress,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_street_address != street_address);
    }

    #[test]
    fn fake_national_identity_number() {
        let national_identity_number = "JR 55 55 55 E";
        let mut rng = rng::get();
        let new_national_identity_number = transform(
            &mut rng,
            national_identity_number,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeNationalIdentityNumber,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_national_identity_number != national_identity_number);
        assert!(national_insurance_number::NATIONAL_INSURANCE_NUMBERS
            .contains(&new_national_identity_number.as_ref()));
    }

    #[test]
    fn fake_email_or_phone_with_phone() {
        let phone_number = "+447822222222";
        let mut rng = rng::get();
        let new_phone_number = transform(
            &mut rng,
            phone_number,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeEmailOrPhone,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_phone_number != phone_number);
        assert!(new_phone_number.starts_with("+4477009"));
        assert_eq!(new_phone_number.len(), 13);
    }
    #[test]
    fn fake_email_or_phone_with_email() {
        let email = "peter@peterson.com";
        let mut rng = rng::get();
        let new_email = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeEmailOrPhone,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_email != email);
        assert!(new_email.contains('@'));
    }

    #[test]
    fn fake_phone_number_gb() {
        let phone_number = "+447822222222";
        let mut rng = rng::get();
        let new_phone_number = transform(
            &mut rng,
            phone_number,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakePhoneNumber,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_phone_number != phone_number);
        assert!(new_phone_number.starts_with("+4477009"));
        assert_eq!(new_phone_number.len(), 13);
    }

    #[test]
    fn fake_phone_number_us() {
        let phone_number = "+16505130514";
        let mut rng = rng::get();
        let new_phone_number = transform(
            &mut rng,
            phone_number,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakePhoneNumber,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_phone_number != phone_number);
        assert!(new_phone_number.starts_with("+1"));
        assert_eq!(new_phone_number.len(), 12);
    }

    #[test]
    fn fake_postcode() {
        let postcode = "NW5 3QQ";
        let mut rng = rng::get();
        let new_postcode = transform(
            &mut rng,
            postcode,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakePostCode,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_postcode, "NW5");
    }

    #[test]
    fn fake_user_name() {
        let user_name = "any user_name";
        let mut rng = rng::get();
        let new_user_name = transform(
            &mut rng,
            user_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeUsername,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_user_name != user_name);
    }

    #[test]
    fn fake_user_name_supports_unique_arg() {
        let user_name = "any user_name";
        let mut rng = rng::get();
        let transformer = &Transformer {
            name: TransformerType::FakeUsername,
            args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
        };
        let new_user_name = transform(
            &mut rng,
            user_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert!(new_user_name != user_name);
        let re = Regex::new(r"^[0-9]+-.*").unwrap();
        assert!(
            re.is_match(&new_user_name),
            "Username {:?} does not have the unique prefix",
            new_user_name
        );
    }

    #[test]
    fn fixed() {
        let url = "any web address";
        let fixed_url = "a very fixed web address";
        let mut rng = rng::get();
        let transformer = &Transformer {
            name: TransformerType::Fixed,
            args: Some(HashMap::from([(
                "value".to_string(),
                fixed_url.to_string(),
            )])),
        };
        let new_url = transform(
            &mut rng,
            url,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_url, fixed_url);
    }
    #[test]
    #[should_panic(expected = "'value' must be present in args for a fixed transformer")]
    fn fixed_panics_if_value_not_provided() {
        let mut rng = rng::get();
        let url = "any web address";
        transform(
            &mut rng,
            url,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Fixed,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
    }

    #[test]
    fn obfuscate_day() {
        let date = "2020-12-12";
        let mut rng = rng::get();
        let obfuscated_date = transform(
            &mut rng,
            date,
            &Type::SingleValue {
                sub_type: SubType::Unknown {
                    underlying_type: "some date or another".to_string(),
                },
            },
            &Transformer {
                name: TransformerType::ObfuscateDay,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(obfuscated_date, "2020-12-01");
    }

    #[test]
    #[should_panic(
        expected = "Invalid date found: \"2020-OHMYGOSH-12\" in table: \"gert_lush_table\". Error: \"input contains invalid characters\""
    )]
    fn obfuscate_day_panics_with_invalid_date() {
        let date = "2020-OHMYGOSH-12";
        let mut rng = rng::get();
        transform(
            &mut rng,
            date,
            &Type::SingleValue {
                sub_type: SubType::Unknown {
                    underlying_type: "some sort of date".to_string(),
                },
            },
            &Transformer {
                name: TransformerType::ObfuscateDay,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
    }

    #[test]
    fn can_deal_with_dates_from_before_christ_because_obviously_we_should_have_to() {
        let date = "0001-08-04 BC";
        let mut rng = rng::get();
        let result = transform(
            &mut rng,
            date,
            &Type::SingleValue {
                sub_type: SubType::Unknown {
                    underlying_type: "some sort of date".to_string(),
                },
            },
            &Transformer {
                name: TransformerType::ObfuscateDay,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(result, "0001-08-01 BC");
    }

    #[test]
    fn scramble_maintains_word_boundaries() {
        let initial_value =
            "Now this is a story all about how my life got flipped turned upside down";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        assert_eq!(new_value.chars().count(), initial_value.chars().count());

        let expected_spaces_count = initial_value.matches(' ').count();
        let actual_spaces_count = new_value.matches(' ').count();
        assert_eq!(actual_spaces_count, expected_spaces_count);
    }

    #[test]
    fn scramble_ignores_punctuation() {
        let initial_value = "ab.?";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        let re = Regex::new(r"^[a-z][a-z]\\.\\?").unwrap();
        assert!(
            !re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn scramble_replaces_digits_with_digits() {
        let initial_value = "ab 12 a1b2";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        let re = Regex::new(r"^[a-z]{2} [0-9]{2} [a-z][0-9][a-z][0-9]").unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn scramble_calculates_unicode_length_correctly() {
        let initial_value = "한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한";
        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        assert_eq!(new_value.chars().count(), initial_value.chars().count());
    }

    #[test]
    fn scramble_deals_with_tabs() {
        let initial_value = "this is a tab\t and another \t.";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        //TODO finish this test
    }

    #[test]
    fn scramble_deals_with_newlines() {
        let initial_value = r#"First line\nSecond line\nThird line\n"#;

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        assert!(!new_value.contains("Second line"));
        assert!(!new_value.contains("Third line"));
    }

    #[test]
    fn scramble_changes_integers_into_integers_only() {
        let initial_value = "123456789";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Integer,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        let re = Regex::new(r"^[0-9]{9}$").unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn can_scramble_array_string_fields() {
        let initial_value = r#"{a,b,"c or d"}"#;
        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::Array {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        let re = Regex::new(r#"^\{"[a-z]","[a-z]","[a-z] [a-z]{2} [a-z]"\}$"#).unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn can_deal_with_commas_inside_values() {
        let initial_value = r#"{"A, or B",C}"#;
        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::Array {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        let re = Regex::new(r#"^\{"[a-z]{2} [a-z]{2} [a-z]","[a-z]"\}$"#).unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn ignores_arrays_if_identity() {
        //TODO currently we have a couple of bugs in parsing around commas inside strings
        let initial_value = "{\"A, B\", \"C\"}";
        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::Array {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Identity,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_value, initial_value);
    }

    #[test]
    fn can_scramble_array_integer_fields() {
        let initial_value = "{1, 22, 444, 5656}";
        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::Array {
                sub_type: SubType::Integer,
            },
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert!(new_value != initial_value);
        let re = Regex::new(r#"^\{[0-9],[0-9]{2},[0-9]{3},[0-9]{4}\}$"#).unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn scramble_blank_maintains_word_boundaries() {
        let initial_value = "Sample Text";

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::ScrambleBlank,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert!(new_value == "______ ____");
    }

    #[test]
    fn scramble_blank_maintains_newlines() {
        let initial_value = r#"One\nTwo\nThree"#;

        let mut rng = rng::get();
        let new_value = transform(
            &mut rng,
            initial_value,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::ScrambleBlank,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert!(new_value == r#"___\n___\n_____"#);
    }

    #[test]
    fn json_array() {
        let json = r#"{"{\\"sender\\": \\"pablo\\"}","{\\"sender\\": \\"barry\\"}"}"#;
        let mut rng = rng::get();
        let new_json = transform(
            &mut rng,
            json,
            &Type::Array {
                sub_type: SubType::Json,
            },
            &Transformer {
                name: TransformerType::EmptyJson,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_json, "{\"{}\",\"{}\"}");
    }

    #[test]
    fn empty_json() {
        let json = "{\"foo\": \"bar\"}";
        let mut rng = rng::get();
        let new_json = transform(
            &mut rng,
            json,
            &Type::SingleValue {
                sub_type: SubType::Unknown {
                    underlying_type: "jsonb".to_string(),
                },
            },
            &Transformer {
                name: TransformerType::EmptyJson,
                args: None,
            },
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );
        assert_eq!(new_json, "{}");
    }

    #[test]
    fn fake_email_with_salt() {
        let email = "john.doe@example.com";
        let mut rng = rng::get();

        // Create transformer
        let transformer = Transformer {
            name: TransformerType::FakeEmail,
            args: None,
        };

        // Test with salt
        let new_email_with_salt = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("test_salt"),
        );

        // Test without salt
        let new_email_without_salt = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            None,
        );

        assert_ne!(
            new_email_with_salt, new_email_without_salt,
            "Same input with and without salt should produce different fake emails"
        );

        // Test with different salt
        let new_email_with_different_salt = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &transformer,
            TABLE_NAME,
            EMPTY_COLUMNS,
            Some("different_salt"),
        );

        assert_ne!(
            new_email_with_salt, new_email_with_different_salt,
            "Same input with different salts should produce different fake emails"
        );
    }
}
