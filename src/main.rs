mod anonymiser;
mod compression_type;
mod file_reader;
mod fixers;
mod opts;
mod parsers;
mod uncompress;

use crate::fixers::fixer;
use crate::fixers::fixer::SortResult;
use crate::opts::{Anonymiser, Opts};
use crate::parsers::strategies::Strategies;
use crate::parsers::strategy_errors::StrategyFileError;
use crate::parsers::strategy_structs::{StrategyInFile, TransformerOverrides};
use colored::Colorize;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;

use parsers::{db_schema, strategy_file};
use structopt::StructOpt;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), std::io::Error> {
    let opt = Opts::from_args();

    match opt.commands {
        Anonymiser::Anonymise {
            input_file,
            output_file,
            strategy_file,
            compress_output,
            allow_potential_pii,
            allow_commercially_sensitive,
            scramble_blank,
        } => {
            let transformer_overrides = TransformerOverrides {
                allow_potential_pii,
                allow_commercially_sensitive,
                scramble_blank,
            };

            anonymiser::anonymise(
                input_file,
                output_file,
                strategy_file,
                compress_output,
                transformer_overrides,
            )?
        }
        Anonymiser::ToCsv {
            output_file,
            strategy_file,
        } => strategy_file::to_csv(&strategy_file, &output_file)?,
        Anonymiser::CheckStrategies {
            strategy_file,
            db_url,
        } => match read_strategy_file(&strategy_file, &db_url) {
            Ok(strategies) => match strategy_differences(strategies, db_url.clone()) {
                Ok(()) => println!("All up to date"),
                Err(err) => {
                    println!("{}", err);
                    if fixer::can_fix(&err) {
                        let retry_command = format!(
                            "anonymiser fix-strategies --db-url={} --strategy-file={}",
                            db_url, strategy_file
                        )
                        .green();
                        println!("But the great news is we can fix at least some of your mess... try running:\n{}", retry_command);
                    } else {
                        println!("Bad news... we currently cannot fix this for you, you'll have to sort it out yourself!");
                    }
                    std::process::exit(1);
                }
            },
            Err(err) => {
                println!("{}", err);
                std::process::exit(1);
            }
        },

        Anonymiser::FixStrategies {
            strategy_file,
            db_url,
        } => match read_strategy_file(&strategy_file, &db_url) {
            Ok(strategies) => match strategy_differences(strategies, db_url) {
                Ok(()) => match fixer::just_sort(&strategy_file) {
                    SortResult::Sorted => {
                        println!("Ok, we've updated that for you, check your diff!")
                    }
                    SortResult::NoChange => {
                        println!("Somehow you got lucky and your file was already sorted perfectly")
                    }
                },
                Err(err) => {
                    println!("{}", err);
                    println!("Ok! lets try and fix some of this!");
                    fixer::fix(&strategy_file, *err);
                    println!("All done, you probably want to run \"check-strategies\" again to make sure");
                }
            },
            Err(err) => {
                println!("{}", err);
                std::process::exit(1);
            }
        },

        Anonymiser::GenerateStrategies {
            strategy_file,
            db_url,
        } => {
            match strategy_differences(Vec::new(), db_url) {
                Ok(()) => println!("All up to date"),
                Err(err) => {
                    if fixer::can_fix(&err) {
                        fixer::fix(&strategy_file, *err);
                        println!("All done, you'll need to set a data_type and transformer for those fields");
                    }
                    std::process::exit(1);
                }
            }
        }
        Anonymiser::Uncompress {
            input_file,
            output_file,
        } => uncompress::uncompress(input_file, output_file).expect("failed to uncompress"),
    }
    Ok(())
}

fn read_strategy_file(strategy_file: &str, db_url: &str) -> Result<Vec<StrategyInFile>, String> {
    match strategy_file::read(strategy_file) {
        Ok(strategies) => Ok(strategies),
        Err(_) => {
            let retry_command = format!(
                "anonymiser generate-strategies --db-url={} --strategy-file={}",
                db_url, strategy_file
            )
            .green();
            Err(format!(
                "Strategy file {} not found. You can use \n{}\nto create an initial file",
                strategy_file, retry_command
            ))
        }
    }
}

fn strategy_differences(
    strategies: Vec<StrategyInFile>,
    db_url: String,
) -> Result<(), Box<StrategyFileError>> {
    let transformer = TransformerOverrides::none();
    let parsed_strategies = Strategies::from_strategies_in_file(strategies, &transformer)
        .map_err(|e| Box::new(StrategyFileError::ValidationError(Box::new(*e))))?;

    let builder = TlsConnector::builder();
    let connector =
        MakeTlsConnector::new(builder.build().expect("should be able to create builder!"));

    let mut client = postgres::Client::connect(&db_url, connector).expect("expected to connect!");
    let db_columns = db_schema::parse(&mut client);
    parsed_strategies
        .validate_against_db(db_columns)
        .map_err(|e| Box::new(StrategyFileError::DbMismatchError(Box::new(e))))?;
    Ok(())
}

#[cfg(test)]
mod test_builders;
