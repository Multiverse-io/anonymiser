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
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

static UNIQUE_INTEGER: AtomicUsize = AtomicUsize::new(0);

fn get_unique() -> usize {
    UNIQUE_INTEGER.fetch_add(1, Ordering::SeqCst)
}

pub fn transform<'line>(
    rng: &mut SmallRng,
    value: &'line str,
    column_type: &Type,
    transformer: &'line Transformer,
    table_name: &str,
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
        return transform_array(rng, value, underlying_type, transformer, table_name);
    }

    let unique = get_unique();

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
        TransformerType::FakeCompanyName => Cow::from(fake_company_name(&transformer.args, unique)),
        TransformerType::FakeEmail => Cow::from(fake_email(&transformer.args, unique)),
        TransformerType::FakeEmailOrPhone => {
            Cow::from(fake_email_or_phone(value, &transformer.args, unique))
        }
        TransformerType::FakeFirstName => Cow::from(FirstName().fake::<String>()),
        TransformerType::FakeFullAddress => Cow::from(fake_full_address()),
        TransformerType::FakeFullName => Cow::from(fake_full_name()),
        TransformerType::FakeIPv4 => Cow::from(IPv4().fake::<String>()),
        TransformerType::FakeLastName => Cow::from(LastName().fake::<String>()),
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
        //TODO not tested VV
        TransformerType::FakeUUID => Cow::from(Uuid::new_v4().to_string()),
    }
}

fn transform_array<'value>(
    rng: &mut SmallRng,
    value: &'value str,
    underlying_type: &SubType,
    transformer: &Transformer,
    table_name: &str,
) -> Cow<'value, str> {
    let quoted_types = vec![SubType::Character, SubType::Json];
    let requires_quotes = quoted_types.contains(underlying_type);

    let sub_type = SingleValue {
        sub_type: underlying_type.clone(),
    };

    let transformed_array = if requires_quotes {
        transform_quoted_array(rng, value, &sub_type, transformer, table_name)
    } else {
        let unsplit_array = &value[1..value.len() - 1];
        unsplit_array
            .split(", ")
            .map(|list_item| transform(rng, list_item, &sub_type, transformer, table_name))
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
            let transformed = transform(rng, &current_word, sub_type, transformer, table_name);
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
    base32::encode(Alphabet::RFC4648 { padding: true }, &random_bytes)
}

fn fake_company_name(args: &Option<HashMap<String, String>>, unique: usize) -> String {
    let new_company_name = CompanyName().fake();
    prepend_unique_if_present(new_company_name, args, unique)
}

fn fake_email(optional_args: &Option<HashMap<String, String>>, unique: usize) -> String {
    let new_email = FreeEmail().fake();
    prepend_unique_if_present(new_email, optional_args, unique)
}

fn fake_email_or_phone(
    current_value: &str,
    optional_args: &Option<HashMap<String, String>>,
    unique: usize,
) -> String {
    if current_value.starts_with("+") && !current_value.contains("@") {
        fake_phone_number(current_value)
    } else {
        fake_email(optional_args, unique)
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

fn fake_full_name() -> String {
    let first: String = FirstName().fake();
    let last: String = LastName().fake();
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
        );
        assert!(new_verification_key != verification_key);
        assert_eq!(new_verification_key.len(), 32);
    }

    #[test]
    fn fake_company_name() {
        let company_name = "any company name";
        let mut rng = rng::get();
        let new_company_name = transform(
            &mut rng,
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeCompanyName,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_company_name != company_name);
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
        );
        assert!(new_company_name != company_name);
    }

    #[test]
    fn fake_email() {
        let email = "any email";
        let mut rng = rng::get();
        let new_email = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeEmail,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_email != email);

        let re = Regex::new(r"^[0-9]+-.*@.*\..*").unwrap();
        assert!(
            !re.is_match(&new_email),
            "Email {:?} should not have the unique prefix",
            new_email
        );
    }

    #[test]
    fn fake_email_with_unique_arg() {
        let email = "rupert@example.com";
        let mut rng = rng::get();
        let transformer = &Transformer {
            name: TransformerType::FakeEmail,
            args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
        };
        let new_email = transform(
            &mut rng,
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            transformer,
            TABLE_NAME,
        );
        assert!(new_email != email);
        let re = Regex::new(r"^[0-9]+-.*@.*\..*").unwrap();
        assert!(
            re.is_match(&new_email),
            "Email {:?} does not have the unique prefix",
            new_email
        );
    }

    #[test]
    fn fake_first_name() {
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
        );
        assert!(new_first_name != first_name);
    }

    #[test]
    fn fake_full_name() {
        let full_name = "any full name";
        let mut rng = rng::get();
        let new_full_name = transform(
            &mut rng,
            full_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeFullName,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_full_name != full_name);
    }

    #[test]
    fn fake_last_name() {
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
        );
        assert!(new_last_name != last_name);
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
        );
        assert!(new_email != email);
        assert!(new_email.contains("@"));
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
        let initial_value = "한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한한";

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
        );
        println!("{new_value}");
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
        );
        assert_eq!(new_json, "{}");
    }
}
