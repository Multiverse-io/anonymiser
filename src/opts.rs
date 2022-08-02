use structopt::StructOpt;
#[derive(Debug, StructOpt)]
#[structopt(name = "Anonymiser", about = "Anonymise your database backups!")]
pub struct Opts {
    #[structopt(subcommand)]
    pub commands: Anonymiser,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "anonymiser")]
pub enum Anonymiser {
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
