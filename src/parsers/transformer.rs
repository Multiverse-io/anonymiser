use crate::parsers::national_insurance_number;
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TransformerType {
    EmptyJson,
    Error,
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

pub fn transform<'line>(value: &'line str, transform: &Transformer) -> String {
    if value == "\\N" {
        return value.to_string();
    }

    let unique = get_unique();
    match transform.name {
        TransformerType::EmptyJson => "{}".to_string(),
        TransformerType::Error => panic!("Error transform still in place"),
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
        //TODO not tested VV
        TransformerType::FakeUUID => Uuid::new_v4().to_string(),
        TransformerType::Fixed => fixed(&transform.args),
        TransformerType::Identity => value.to_string(),
        TransformerType::ObfuscateDay => obfuscate_day(value),
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
    //TODO currently we dont validate, this is free text so they can enter anything at all, so im
    //not bothering with us vs uk, there dont seem to be any us social sec numbers in the DB
    //currently
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

fn fixed(args: &Option<HashMap<String, String>>) -> String {
    let value = args
        .as_ref()
        .and_then(|a| a.get("value"))
        .expect("Value must be present in args for a fixed transformer");
    return value.to_string();
}

fn obfuscate_day(value: &str) -> String {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .expect(&format!("Invalid date found: {}", value));
    let new_date = date.with_day(1).unwrap();
    return new_date.to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::national_insurance_number;
    use regex::Regex;

    #[test]
    fn nul_is_not_transformed() {
        let null = "\\N";
        let new_null = transform(
            null,
            &Transformer {
                name: TransformerType::Scramble,
                args: None,
            },
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
        );
        assert!(new_first_name == first_name);
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
        );
        assert!(new_postcode != postcode);
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
        );
        assert_eq!(obfuscated_date, "2020-12-01");
    }

    #[test]
    #[should_panic(expected = "Invalid date found: 2020-OHMYGOSH-12")]
    fn obfuscate_day_panics_with_invalid_date() {
        let date = "2020-OHMYGOSH-12";
        transform(
            date,
            &Transformer {
                name: TransformerType::ObfuscateDay,
                args: None,
            },
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
        );
        assert!(new_value != initial_value);
        assert_eq!(new_value.len(), initial_value.len());
    }
}
