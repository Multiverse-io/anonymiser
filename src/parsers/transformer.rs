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
use rand::SeedableRng;
use rand::{rngs::SmallRng, Rng};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

static UNIQUE_INTEGER: AtomicUsize = AtomicUsize::new(0);

fn get_unique() -> usize {
    UNIQUE_INTEGER.fetch_add(1, Ordering::SeqCst)
}

pub fn transform<'line>(
    value: &'line str,
    column_type: &Type,
    transformer: &Transformer,
    table_name: &str,
) -> String {
    if ["\\N", "deleted"].contains(&value) {
        return value.to_string();
    }

    if transformer.name == TransformerType::Identity {
        return value.to_string();
    }

    if let Array {
        sub_type: underlying_type,
    } = column_type
    {
        return transform_array(value, underlying_type, transformer, table_name);
    }

    let unique = get_unique();
    match transformer.name {
        TransformerType::Error => {
            panic!("Error transform still in place for table: {}", table_name)
        }
        TransformerType::EmptyJson => "{}".to_string(),
        TransformerType::FakeBase16String => fake_base16_string(),
        TransformerType::FakeBase32String => fake_base32_string(),
        TransformerType::FakeCity => CityName().fake(),
        TransformerType::FakeCompanyName => fake_company_name(&transformer.args, unique),
        TransformerType::FakeEmail => fake_email(&transformer.args, unique),
        TransformerType::FakeFirstName => FirstName().fake(),
        TransformerType::FakeFullAddress => fake_full_address(),
        TransformerType::FakeFullName => fake_full_name(),
        TransformerType::FakeIPv4 => IPv4().fake(),
        TransformerType::FakeLastName => LastName().fake(),
        TransformerType::FakeNationalIdentityNumber => fake_national_identity_number(),
        TransformerType::FakePostCode => fake_postcode(value),
        TransformerType::FakePhoneNumber => fake_phone_number(value),
        TransformerType::FakeStreetAddress => fake_street_address(),
        TransformerType::FakeState => StateName().fake(),
        TransformerType::FakeUsername => fake_username(&transformer.args, unique),
        //TODO not tested VV
        TransformerType::FakeUUID => Uuid::new_v4().to_string(),
        TransformerType::Fixed => fixed(&transformer.args, table_name),
        TransformerType::Identity => value.to_string(),
        TransformerType::ObfuscateDay => obfuscate_day(value, table_name),
        TransformerType::Scramble => scramble(value),
    }
}

fn transform_array(
    value: &str,
    underlying_type: &SubType,
    transformer: &Transformer,
    table_name: &str,
) -> String {
    let is_string_array = underlying_type == &SubType::Character;
    let mut unsplit_array = value.to_string();

    let sub_type = SingleValue {
        sub_type: underlying_type.clone(),
    };
    unsplit_array.remove(0);
    unsplit_array.pop();

    let array: Vec<String> = unsplit_array
        .split(", ")
        .map(|list_item| {
            if is_string_array {
                let mut list_item_without_enclosing_quotes = list_item.to_string();
                list_item_without_enclosing_quotes.remove(0);
                list_item_without_enclosing_quotes.pop();
                let transformed = transform(
                    &list_item_without_enclosing_quotes,
                    &sub_type,
                    transformer,
                    table_name,
                );

                format!("\"{}\"", transformed)
            } else {
                transform(list_item, &sub_type, transformer, table_name)
            }
        })
        .collect();

    return format!("{{{}}}", array.join(", "));
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
        return format!("{}-{}", unique, new_value);
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
        return format!("+447700{0}", random);
    } else {
        let area_code = rng.gen_range(200..999);
        let rest = rng.gen_range(1000000..9999999);
        return format!("+1{}{}", area_code, rest);
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

fn fixed(args: &Option<HashMap<String, String>>, table_name: &str) -> String {
    let value = args
        .as_ref()
        .and_then(|a| a.get("value"))
        .unwrap_or_else(|| {
            panic!(
                "'value' must be present in args for a fixed transformer in table: '{}'\ngot: '{:?}'",
                table_name, args,
            )
        });
    value.to_string()
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

fn scramble(original_value: &str) -> String {
    let mut chars = original_value.chars();
    let mut output_buf = String::with_capacity(original_value.len());

    let mut rng =
        SmallRng::from_rng(rand::thread_rng()).unwrap_or_else(|_| SmallRng::from_entropy());

    while let Some(current_char) = chars.next() {
        if current_char == '\\' {
            //The string contains a control character like \t \r \n
            output_buf.push(current_char);
            if let Some(c) = chars.next() {
                output_buf.push(c);
            }
        } else if current_char == ' ' {
            output_buf.push(current_char);
        } else if current_char.is_ascii_digit() {
            let new_char = rng.gen_range(b'0'..=b'9') as char;
            output_buf.push(new_char);
        } else {
            let new_char = rng.gen_range(b'a'..=b'z') as char;
            output_buf.push(new_char);
        }
    }

    output_buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::national_insurance_number;
    use regex::Regex;

    const TABLE_NAME: &str = "gert_lush_table";
    #[test]
    fn null_is_not_transformed() {
        let null = "\\N";
        let new_null = transform(
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
        let new_deleted = transform(
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
        let new_first_name = transform(
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
        let new_verification_key = transform(
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
        let new_verification_key = transform(
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
        let new_company_name = transform(
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
        let new_company_name = transform(
            company_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeCompanyName,
                args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
            },
            TABLE_NAME,
        );
        assert!(new_company_name != company_name);
    }

    #[test]
    fn fake_email() {
        let email = "any email";
        let new_email = transform(
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
        let new_email = transform(
            email,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeEmail,
                args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
            },
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
        let new_first_name = transform(
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
        let new_full_name = transform(
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
        let new_last_name = transform(
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
        let new_street_address = transform(
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
        let new_national_identity_number = transform(
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
    fn fake_phone_number_gb() {
        let phone_number = "+447822222222";
        let new_phone_number = transform(
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
        let new_phone_number = transform(
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
        let new_postcode = transform(
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
        let new_user_name = transform(
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
        let new_user_name = transform(
            user_name,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::FakeUsername,
                args: Some(HashMap::from([("unique".to_string(), "true".to_string())])),
            },
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
        let new_url = transform(
            url,
            &Type::SingleValue {
                sub_type: SubType::Character,
            },
            &Transformer {
                name: TransformerType::Fixed,
                args: Some(HashMap::from([(
                    "value".to_string(),
                    fixed_url.to_string(),
                )])),
            },
            TABLE_NAME,
        );
        assert_eq!(new_url, fixed_url);
    }
    #[test]
    #[should_panic(expected = "'value' must be present in args for a fixed transformer")]
    fn fixed_panics_if_value_not_provided() {
        let url = "any web address";
        transform(
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
        let obfuscated_date = transform(
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
        transform(
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
        let result = transform(
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

        let new_value = transform(
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

        let new_value = transform(
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

        let new_value = transform(
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

        let new_value = transform(
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

        let new_value = transform(
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
    fn scramble_changes_integers_into_integers_only() {
        let initial_value = "123456789";

        let new_value = transform(
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
        let initial_value = "{\"A\", \"B\"}";
        let new_value = transform(
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
        let re = Regex::new(r#"^\{"[a-z]", "[a-z]"\}$"#).unwrap();
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
        let new_value = transform(
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
        let new_value = transform(
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
        let re = Regex::new(r#"^\{[0-9], [0-9]{2}, [0-9]{3}, [0-9]{4}\}$"#).unwrap();
        assert!(
            re.is_match(&new_value),
            "new value: \"{}\" does not contain same digit / alphabet structure as input",
            new_value
        );
    }

    #[test]
    fn empty_json() {
        let json = "{\"foo\": \"bar\"}";
        let new_json = transform(
            json,
            &Type::SingleValue {
                sub_type: SubType::Character,
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
