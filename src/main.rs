mod file_reader;
mod parsers;
use crate::parsers::strategy_structs::{
    MissingColumns, SimpleColumn, Strategies, TransformerOverrides,
};
use itertools::Itertools;
use parsers::{db_schema, strategy_file_reader, strategy_validator};
use postgres::{Client, NoTls};
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "Anonymiser", about = "Anonymise your database backups!")]
pub struct Opts {
    #[structopt(subcommand)]
    commands: Anonymiser,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "anonymiser")]
enum Anonymiser {
    Anonymise {
        #[structopt(short, long, default_value = "./clear_text_dump.sql")]
        input_file: String,
        #[structopt(short, long, default_value = "./output.sql")]
        output_file: String,
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,
        /// Does not transform PotentiallPii data types
        #[structopt(long)]
        allow_potential_pii: bool,
        /// Does not transform Commercially sensitive data types
        #[structopt(long)]
        allow_commercially_sensitive: bool,
    },

    ToCsv {
        #[structopt(short, long, default_value = "./output.csv")]
        output_file: String,
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,
    },

    CheckStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long)]
        fix: bool,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },

    GenerateStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },
}

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
                allow_potential_pii: allow_potential_pii,
                allow_commercially_sensitive: allow_commercially_sensitive,
            };

            let strategies = strategy_file_reader::read(&strategy_file, transformer_overrides);
            file_reader::read(input_file, output_file, &strategies)?;
        }
        Anonymiser::ToCsv {
            output_file,
            strategy_file,
        } => {
            strategy_file_reader::to_csv(&strategy_file, &output_file)?;
        }
        Anonymiser::CheckStrategies {
            strategy_file,
            fix,
            db_url,
        } => {
            let transformer = TransformerOverrides::default();
            let strategies = strategy_file_reader::read(&strategy_file, transformer);
            match strategy_differences(&strategies, db_url) {
                Ok(()) => println!("All up to date"),
                Err(missing_columns) => {
                    let message = format_missing_columns(&strategy_file, &missing_columns);
                    println!("{}", message);
                    if fix && fixable(&missing_columns) {
                        println!("But the great news is that we're going to try and fix some of this!...");
                        fix_missing_columns(&strategy_file, missing_columns);
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
            match strategy_differences(&HashMap::new(), db_url) {
                Ok(()) => println!("All up to date"),
                Err(missing_columns) => {
                    if fixable(&missing_columns) {
                        fix_missing_columns(&strategy_file, missing_columns);
                        println!("All done, you'll need to set a data_type and transformer for those fields");
                    }
                    std::process::exit(1);
                }
            }
        }
    }
    return Ok(());
}

fn fixable(missing_columns: &MissingColumns) -> bool {
    return missing_columns.missing_from_strategy_file.is_some()
        && missing_columns
            .missing_from_strategy_file
            .as_ref()
            .unwrap()
            .len()
            > 0;
}

fn fix_missing_columns(strategy_file: &str, missing_columns: MissingColumns) -> () {
    match missing_columns.missing_from_strategy_file {
        Some(missing) => {
            strategy_file_reader::append_to_file(&strategy_file, missing)
                .expect("Unable to write to file :(");
        }
        None => (),
    };
}

fn format_missing_columns(strategy_file: &str, missing_columns: &MissingColumns) -> String {
    let mut message = "".to_string();

    match &missing_columns.unanonymised_pii {
        Some(missing) => {
            let missing_list = missing_to_message(&missing);
            message.push_str(&format!(
                "Some fields are tagged as being PII but do not have anonymising transformers set. ({})\n\t{}\nPlease add valid transformers!\n\n",
                strategy_file, missing_list
            ))
        }
        None => (),
    }

    match &missing_columns.error_transformer_types {
        Some(missing) => {
            let missing_list = missing_to_message(&missing);
            message.push_str(&format!(
                "Some fields still have 'Error' transformer types ({})\n\t{}\nPlease add valid transformers!\n\n",
                strategy_file, missing_list
            ))
        }
        None => (),
    }

    match &missing_columns.unknown_data_types {
        Some(missing) => {
            let missing_list = missing_to_message(&missing);
            message.push_str(&format!(
                "Some fields still have 'Unknown' data types ({})\n\t{}\nPlease add valid data types!\n\n",
                strategy_file, missing_list
            ))
        }
        None => (),
    }
    match &missing_columns.missing_from_db {
        Some(missing) => {
            let missing_list = missing_to_message(&missing);
            message.push_str(&format!(
                "Some fields are in the strategies file ({}) but not the database!\n\t{}\n",
                strategy_file, missing_list
            ))
        }
        None => (),
    }

    match &missing_columns.missing_from_strategy_file {
        Some(missing) => {
            let missing_list = missing_to_message(&missing);
            message.push_str(&format!(
                "Some fields are missing from strategies file ({})\n\t{}\n",
                strategy_file, missing_list
            ))
        }
        None => (),
    }

    return message;
}

fn missing_to_message(missing: &Vec<SimpleColumn>) -> String {
    return missing
        .iter()
        .map(|c| format!("{} => {}", &c.table_name, &c.column_name))
        .sorted()
        .join("\n\t");
}

fn strategy_differences(strategies: &Strategies, db_url: String) -> Result<(), MissingColumns> {
    let mut conn = Client::connect(&db_url, NoTls).expect("expected connection to succeed");
    let db_columns = db_schema::parse(&mut conn);
    return strategy_validator::validate(&strategies, db_columns);
}
