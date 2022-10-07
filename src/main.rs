mod anonymiser;
mod file_reader;
mod fixers;
mod opts;
mod parsers;
mod uncompress;

use crate::fixers::fixer;
use crate::opts::{Anonymiser, Opts};
use crate::parsers::strategies::Strategies;
use crate::parsers::strategy_errors::StrategyFileError;
use crate::parsers::strategy_structs::{StrategyInFile, TransformerOverrides};
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
        } => {
            let transformer_overrides = TransformerOverrides {
                allow_potential_pii,
                allow_commercially_sensitive,
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
            fix: fix_flag,
            strategy_file,
            db_url,
        } => {
            let strategies = strategy_file::read(&strategy_file).unwrap_or_else(|_| Vec::new());

            match strategy_differences(strategies, db_url) {
                Ok(()) => println!("All up to date"),
                Err(err) => {
                    println!("{}", err);
                    if fix_flag && fixer::can_fix(&err) {
                        println!("But the great news is that we're going to try and fix some of this!...");
                        fixer::fix_columns(&strategy_file, err);
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
            match strategy_differences(Vec::new(), db_url) {
                Ok(()) => println!("All up to date"),
                Err(err) => {
                    if fixer::can_fix(&err) {
                        fixer::fix_columns(&strategy_file, err);
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

fn strategy_differences(
    strategies: Vec<StrategyInFile>,
    db_url: String,
) -> Result<(), StrategyFileError> {
    let transformer = TransformerOverrides::none();
    let parsed_strategies = Strategies::from_strategies_in_file(strategies, &transformer)?;
    let builder = TlsConnector::builder();
    let connector =
        MakeTlsConnector::new(builder.build().expect("should be able to create builder!"));

    let mut client = postgres::Client::connect(&db_url, connector).expect("expected to connect!");
    let db_columns = db_schema::parse(&mut client);
    parsed_strategies.validate_against_db(db_columns)?;
    Ok(())
}

#[cfg(test)]
mod test_builders;
