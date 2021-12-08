mod file_reader;
mod parsers;
use parsers::strategy_file;
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
        #[structopt(short, long, default_value = "./strategies.json")]
        strategies_file: String,
    },
}

fn main() -> Result<(), std::io::Error> {
    let opt = Opts::from_args();

    println!("{:?}", opt);
    match opt.commands {
        Anonymiser::Anonymise {
            input_file,
            output_file,
            strategies_file,
        } => {
            let strategies = strategy_file::parse(&strategies_file);
            file_reader::read(input_file, output_file, &strategies)?;
        }
    }
    return Ok(());
}
