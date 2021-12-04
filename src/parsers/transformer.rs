use crate::strategy_file::Transformer;
use crate::strategy_file::TransformerType;
use chrono::{Datelike, NaiveDate};
use fake::faker::address::en::*;
use fake::faker::company::en::*;
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

static UNIQUE_INTEGER: AtomicUsize = AtomicUsize::new(0);

fn get_unique() -> usize {
    return UNIQUE_INTEGER.fetch_add(1, Ordering::SeqCst);
}

pub fn transform<'line>(value: &'line str, transform: &Transformer) -> String {
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
        TransformerType::FakePostCode => PostCode().fake(),
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
    //TODO Fake uk phone ranges - https://www.ofcom.org.uk/phones-telecoms-and-internet/information-for-industry/numbering/numbers-for-drama
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
    use regex::Regex;

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
    fn fake_postcode() {
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
