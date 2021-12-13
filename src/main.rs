mod file_reader;
mod parsers;
use parsers::strategy_file;
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

    GenerateStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },
}

fn main() -> Result<(), std::io::Error> {
    let opt = Opts::from_args();

    println!("{:?}", opt);
    match opt.commands {
        Anonymiser::Anonymise {
            input_file,
            output_file,
            strategy_file,
        } => {
            let strategies = strategy_file::parse(strategy_file);
            file_reader::read(input_file, output_file, &strategies)?;
        }
        Anonymiser::GenerateStrategies {
            strategy_file,
            db_url,
        } => {
            let strategies = strategy_file::parse(strategy_file);
            let mut conn = Client::connect(
                "postgresql://postgres:postgres@localhost:5432/postgres",
                NoTls,
            )
            .expect("expected connection to succeed");
            strategy_file::generate(strategies, &mut conn);
        }
    }
    return Ok(());
}
