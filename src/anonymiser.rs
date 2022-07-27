use crate::file_reader;
use crate::parsers::strategy_file_reader;
use crate::parsers::strategy_structs::TransformerOverrides;

pub fn anonymise(
    input_file: String,
    output_file: String,
    strategy_file: String,
    transformer_overrides: TransformerOverrides,
) -> Result<(), std::io::Error> {
    match strategy_file_reader::read(&strategy_file, transformer_overrides) {
        Ok(strategies) => {
            file_reader::read(input_file, output_file, &strategies)?;
            return Ok(());
        }
        Err(_) => {
            panic!("Strategy file '{}' does not exist", strategy_file)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres::Client;
    use postgres::NoTls;
    use std::process::Command;

    #[test]
    #[should_panic(expected = "Strategy file 'non_existing_strategy_file.json' does not exist")]
    fn panics_if_strategy_file_is_missing() {
        assert!(anonymise(
            "test_files/dump_file.sql".to_string(),
            "test_files/results.sql".to_string(),
            "non_existing_strategy_file.json".to_string(),
            TransformerOverrides::default(),
        )
        .is_ok());
    }

    #[test]
    #[should_panic(expected = "Input file 'non_existing_input_file.sql' does not exist")]
    fn panics_if_input_file_is_missing() {
        assert!(anonymise(
            "non_existing_input_file.sql".to_string(),
            "test_files/results.sql".to_string(),
            "test_files/strategy.json".to_string(),
            TransformerOverrides::default(),
        )
        .is_ok());
    }

    #[test]
    fn successfully_transforms() {
        assert!(anonymise(
            "test_files/dump_file.sql".to_string(),
            "test_files/results.sql".to_string(),
            "test_files/strategy.json".to_string(),
            TransformerOverrides::default(),
        )
        .is_ok());

        let db_url = "postgresql://postgres:postgres@localhost";
        let postgres = format!("{}/postgres", db_url);
        let mut conn = Client::connect(&postgres, NoTls).expect("expected connection to succeed");

        conn.simple_query("drop database if exists anonymiser_test")
            .unwrap();
        conn.simple_query("create database anonymiser_test")
            .unwrap();

        let result = Command::new("psql")
            .arg(format!("{}/anonymiser_test", db_url))
            .arg("-f")
            .arg("test_files/results.sql")
            .arg("-v")
            .arg("ON_ERROR_STOP=1")
            .output()
            .expect("failed!");

        assert!(
            result.status.success(),
            "failed to restore backup:\n{:?}",
            String::from_utf8(result.stderr).unwrap()
        );
    }
}
