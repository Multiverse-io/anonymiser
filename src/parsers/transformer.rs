use crate::parsers::national_insurance_number;
use base16::encode_lower;
use base32::Alphabet;
use chrono::{Datelike, NaiveDate};
use core::ops::Range;
use fake::faker::address::en::*;
use fake::faker::company::en::*;
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

static UNIQUE_INTEGER: AtomicUsize = AtomicUsize::new(0);

fn get_unique() -> usize {
    return UNIQUE_INTEGER.fetch_add(1, Ordering::SeqCst);
}

//TODO
//Postcode too long (should be 8 max)

//ERROR:  value too long for type character varying(255)
//CONTEXT:  COPY timeline_items, line 116588, column title: "z0kHB986epbDuxe9bjDtsBjRKYTu78ayZIpf7SpAJHEIjHlvh0T1GBJhGUel3xCrcYmVJ6Jp8P7qBARBgasIhXaTflkf3zISTbEc..."

//ERROR:  new row for relation "users_including_deactivated" violates check constraint "deactivated_users_cannot_have_real_passwords"

//ERROR:  could not create unique index "auth_tokens_token_index"
//DETAIL:  Key (token)=(Redacted 🤐) is duplicated.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TransformerType {
    EmptyJson,
    Error,
    FakeBase16String,
    FakeBase32String,
    FakeCity,
    FakeCompanyName,
    FakeEmail,
    FakeFirstName,
    FakeFullAddress,
    FakeFullName,
    FakeIPv4,
    FakeLastName,
    FakeNationalIdentityNumber,
    FakePhoneNumber,
    FakePostCode,
    FakeState,
    FakeStreetAddress,
    FakeUsername,
    FakeUUID,
    Fixed,
    Identity,
    ObfuscateDay,
    Redact,
    Scramble,
    Test,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transformer {
    pub name: TransformerType,
    pub args: Option<HashMap<String, String>>,
}

pub fn transform<'line>(value: &'line str, transform: &Transformer, table_name: &str) -> String {
    if value == "\\N" {
        return value.to_string();
    }

    let unique = get_unique();
    match transform.name {
        TransformerType::EmptyJson => "{}".to_string(),
        TransformerType::Error => {
            panic!("Error transform still in place for table: {}", table_name)
        }
        TransformerType::FakeBase16String => fake_base16_string(),
        TransformerType::FakeBase32String => fake_base32_string(),
        TransformerType::FakeCity => CityName().fake(),
        TransformerType::FakeCompanyName => CompanyName().fake(),
        TransformerType::FakeEmail => fake_email(&transform.args, unique),
        TransformerType::FakeFirstName => FirstName().fake(),
        TransformerType::FakeFullAddress => fake_full_address(),
        TransformerType::FakeFullName => fake_full_name(),
        TransformerType::FakeIPv4 => IPv4().fake(),
        TransformerType::FakeLastName => LastName().fake(),
        TransformerType::FakeNationalIdentityNumber => fake_national_identity_number(),
        TransformerType::FakePostCode => PostCode().fake(),
        TransformerType::FakePhoneNumber => fake_phone_number(value),
        TransformerType::FakeStreetAddress => fake_street_address(),
        TransformerType::FakeState => StateName().fake(),
        TransformerType::FakeUsername => Username().fake(),
        //TODO not tested VV
        TransformerType::FakeUUID => Uuid::new_v4().to_string(),
        TransformerType::Fixed => fixed(&transform.args, table_name),
        TransformerType::Identity => value.to_string(),
        TransformerType::ObfuscateDay => obfuscate_day(value, table_name),
        TransformerType::Redact => format!("Redacted {}", '\u{1F910}'),
        TransformerType::Scramble => scramble(value),
        TransformerType::Test => "TestData".to_string(),
    }
}

fn fake_email(optional_args: &Option<HashMap<String, String>>, unique: usize) -> String {
    let unique_value = optional_args
        .as_ref()
        .and_then(|a| a.get("unique"))
        .map_or_else(|| false, |u| u == "true");

    if unique_value {
        let new_email: String = FreeEmail().fake();
        return format!("{}-{}", unique, new_email);
    } else {
        return FreeEmail().fake();
    }
}

//TODO this is pretty naive, we probably want to at least keep the word count?
fn scramble(original_value: &str) -> String {
    let length = original_value.len();
    return thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
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
    return format!("{} {}", first, last);
}

fn fake_national_identity_number() -> String {
    //TODO currently this is free text so they can enter anything at all,
    //so im not bothering with us vs uk,
    //there dont seem to be any us social sec numbers in the DB currently
    return national_insurance_number::random();
}

//https://www.ofcom.org.uk/phones-telecoms-and-internet/information-for-industry/numbering/numbers-for-drama
static UK_FAKE_MOBILE_RANGE: Range<i32> = 900000..960999;

fn fake_phone_number(current_value: &str) -> String {
    let mut rng = rand::thread_rng();
    if current_value.starts_with("+447") {
        let random = rng.gen_range(UK_FAKE_MOBILE_RANGE.clone());
        return format!("+447700{0}", random);
    } else {
        let area_code = rng.gen_range(200..999);
        let rest = rng.gen_range(1000000..9999999);
        return format!("+1{}{}", area_code, rest);
    }
}

fn fake_base16_string() -> String {
    let random_bytes = rand::thread_rng().gen::<[u8; 16]>();
    return base16::encode_lower(&random_bytes);
}

fn fake_base32_string() -> String {
    let random_bytes = rand::thread_rng().gen::<[u8; 16]>();
    return base32::encode(Alphabet::RFC4648 { padding: true }, &random_bytes);
}

fn fixed(args: &Option<HashMap<String, String>>, table_name: &str) -> String {
    let value = args.as_ref().and_then(|a| a.get("value")).expect(&format!(
        "Value must be present in args for a fixed transformer in table: {}",
        table_name,
    ));
    return value.to_string();
}

fn obfuscate_day(value: &str, table_name: &str) -> String {
    match NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Ok(date) => {
            let new_date = date.with_day(1).unwrap();
            return new_date.to_string();
        }
        Err(err) => {
            return value
                .strip_suffix(" BC")
                .and_then(|trimmed| {
                    return NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                        .ok()
                        .and_then(|re_parsed| {
                            let bc_date_years = 0 - re_parsed.year();
                            return re_parsed
                                .with_year(bc_date_years)
                                .and_then(|d| d.with_day(1));
                        });
                })
                .expect(
                    format!(
                        "Invalid date found: \"{}\" in table: \"{}\". Error: \"{}\"",
                        value, table_name, err
                    )
                    .as_ref(),
                )
                .to_string();
        }
    }
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
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
        );
        assert_eq!(new_null, null);
    }

    #[test]
    fn identity() {
        let first_name = "any first name";
        let new_first_name = transform(
            first_name,
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
            &Transformer {
                name: TransformerType::FakeCompanyName,
                args: None,
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
            &Transformer {
                name: TransformerType::FakePhoneNumber,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_phone_number != phone_number);
        print!("{:?}", new_phone_number);
        assert!(new_phone_number.starts_with("+1"));
        assert_eq!(new_phone_number.len(), 12);
    }

    #[test]
    fn fake_postcode() {
        //TODO no idea what this is, some us thing? (e.g. "71746-3648")
        let postcode = "any postcode";
        let new_postcode = transform(
            postcode,
            &Transformer {
                name: TransformerType::FakePostCode,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_postcode != postcode);
    }

    #[test]
    fn fake_user_name() {
        let user_name = "any user_name";
        let new_user_name = transform(
            user_name,
            &Transformer {
                name: TransformerType::FakeUsername,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_user_name != user_name);
    }

    #[test]
    fn fixed() {
        let url = "any web address";
        let fixed_url = "a very fixed web address";
        let new_url = transform(
            url,
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
    #[should_panic(expected = "Value must be present in args for a fixed transformer")]
    fn fixed_panics_if_value_not_provided() {
        let url = "any web address";
        transform(
            url,
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
        transform(
            date,
            &Transformer {
                name: TransformerType::ObfuscateDay,
                args: None,
            },
            TABLE_NAME,
        );
    }

    #[test]
    fn redact() {
        let street_address = "any street_address";
        let redacted_street_address = transform(
            street_address,
            &Transformer {
                name: TransformerType::Redact,
                args: None,
            },
            TABLE_NAME,
        );
        assert_eq!(redacted_street_address, "Redacted 🤐");
    }

    #[test]
    fn scramble_returns_random_string_of_same_length() {
        let initial_value = "This is a story all about how my life got flipped, turned upside down";
        let new_value = transform(
            initial_value,
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
            TABLE_NAME,
        );
        assert!(new_value != initial_value);
        assert_eq!(new_value.len(), initial_value.len());
    }
}
