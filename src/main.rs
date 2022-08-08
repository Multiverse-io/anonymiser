mod anonymiser;
mod file_reader;
use std::fmt::Write;
mod fixer;
mod opts;
mod parsers;
use crate::opts::{Anonymiser, Opts};
use crate::parsers::strategies::Strategies;
use crate::parsers::strategy_structs::{MissingColumns, SimpleColumn, TransformerOverrides};
use itertools::Itertools;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;

use parsers::{db_schema, strategy_file};
use structopt::StructOpt;

fn main() -> Result<(), std::io::Error> {
    let opt = Opts::from_args();

    match opt.commands {
        Anonymiser::Anonymise {
            input_file,
            output_file,
            strategy_file,
            allow_potential_pii,
            allow_commercially_sensitive,
        } => {
            let transformer_overrides = TransformerOverrides {
                allow_potential_pii,
                allow_commercially_sensitive,
            };
            return anonymiser::anonymise(
                input_file,
                output_file,
                strategy_file,
                transformer_overrides,
            );
        }
        Anonymiser::ToCsv {
            output_file,
            strategy_file,
        } => strategy_file::to_csv(&strategy_file, &output_file)?,
        Anonymiser::CheckStrategies {
            fix: fix_flag,
            strategy_file,
            db_url,
        } => {
            let transformer = TransformerOverrides::none();
            let strategies = strategy_file::read(&strategy_file, transformer)
                .unwrap_or_else(|_| Strategies::new());

            match strategy_differences(&strategies, db_url) {
                Ok(()) => println!("All up to date"),
                Err(missing_columns) => {
                    let message = format_missing_columns(&strategy_file, &missing_columns);
                    println!("{}", message);
                    if fix_flag && fixer::can_fix(&missing_columns) {
                        println!("But the great news is that we're going to try and fix some of this!...");
                        fixer::fix_columns(&strategy_file, missing_columns);
                        println!("All done, you'll need to set a data_type and transformer for those fields");
                    }
                    std::process::exit(1);
                }
            }
        }
        Anonymiser::GenerateStrategies {
            strategy_file,
            db_url,
        } => {
            match strategy_differences(&Strategies::new(), db_url) {
                Ok(()) => println!("All up to date"),
                Err(missing_columns) => {
                    if fixer::can_fix(&missing_columns) {
                        fixer::fix_columns(&strategy_file, missing_columns);
                        println!("All done, you'll need to set a data_type and transformer for those fields");
                    }
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn format_missing_columns(strategy_file: &str, missing_columns: &MissingColumns) -> String {
    let mut message = "".to_string();

    if !missing_columns.unanonymised_pii.is_empty() {
        let missing_list = missing_to_message(&missing_columns.unanonymised_pii);
        write!(message,
                "Some fields are tagged as being PII but do not have anonymising transformers set. ({})\n\t{}\nPlease add valid transformers!\n\n",
                strategy_file, missing_list
            ).unwrap()
    }

    if !missing_columns.error_transformer_types.is_empty() {
        let missing_list = missing_to_message(&missing_columns.error_transformer_types);
        write!(message, "Some fields still have 'Error' transformer types ({})\n\t{}\nPlease add valid transformers!\n\n",
                strategy_file, missing_list
            ).unwrap()
    }

    if !missing_columns.unknown_data_categories.is_empty() {
        let missing_list = missing_to_message(&missing_columns.unknown_data_categories);
        write!(message,
                "Some fields still have 'Unknown' data types ({})\n\t{}\nPlease add valid data types!\n\n",
                strategy_file, missing_list
            ).unwrap()
    }
    if !missing_columns.missing_from_db.is_empty() {
        let missing_list = missing_to_message(&missing_columns.missing_from_db);
        write!(
            message,
            "Some fields are in the strategies file ({}) but not the database!\n\t{}\n",
            strategy_file, missing_list
        )
        .unwrap()
    }

    if !missing_columns.missing_from_strategy_file.is_empty() {
        let missing_list = missing_to_message(&missing_columns.missing_from_strategy_file);
        write!(
            message,
            "Some fields are missing from strategies file ({})\n\t{}\n",
            strategy_file, missing_list
        )
        .unwrap()
    }

    message
}

fn missing_to_message(missing: &[SimpleColumn]) -> String {
    return missing
        .iter()
        .map(|c| format!("{} => {}", &c.table_name, &c.column_name))
        .sorted()
        .join("\n\t");
}

fn strategy_differences(strategies: &Strategies, db_url: String) -> Result<(), MissingColumns> {
    let builder = TlsConnector::builder();
    let connector =
        MakeTlsConnector::new(builder.build().expect("should be able to create builder!"));

    let mut client = postgres::Client::connect(&db_url, connector).expect("expected to connect!");
    let db_columns = db_schema::parse(&mut client);
    strategies.validate(db_columns)
}

#[cfg(test)]
mod test_builders;
