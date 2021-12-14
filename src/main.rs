mod file_reader;
mod parsers;
use crate::parsers::strategy_structs::{MissingColumns, SimpleColumn, Strategies};
use itertools::Itertools;
use parsers::{db_schema, strategy_file, strategy_validator};
use postgres::{Client, NoTls};
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
        } => {
            let strategies = strategy_file::parse(&strategy_file);
            file_reader::read(input_file, output_file, &strategies)?;
        }
        Anonymiser::CheckStrategies {
            strategy_file,
            fix,
            db_url,
        } => {
            let strategies = strategy_file::parse(&strategy_file);
            match strategy_differences(&strategies, db_url) {
                Ok(()) => println!("All up to date"),
                Err(missing_columns) => {
                    let message = format_missing_columns(&strategy_file, &missing_columns);
                    println!("{}", message);
                    if fix {
                        println!("But the great news is that we're going to try and fix this!...");
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
            let strategies = strategy_file::parse(&strategy_file);
            let _result = strategy_differences(&strategies, db_url);
        }
    }
    return Ok(());
}

fn fix_missing_columns(strategy_file: &str, missing_columns: MissingColumns) -> () {
    match missing_columns.missing_from_strategy_file {
        Some(missing) => {
            strategy_file::append_to_file(&strategy_file, missing)
                .expect("Unable to write to file :(");
        }
        None => (),
    };
}

fn format_missing_columns(strategy_file: &str, missing_columns: &MissingColumns) -> String {
    let mut message = "".to_string();
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
