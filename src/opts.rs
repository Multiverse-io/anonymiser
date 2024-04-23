use crate::compression_type::CompressionType;
use std::path::PathBuf;
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
        /// Path to the strategy.json file
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,
        /// Either just a flag
        /// e.g. '--compress-output' in which case it defaults to zstd
        /// or with a        /// compression type e.g. '--compress-output zstd' or '--compress-output gzip'
        /// compr
        #[structopt(short, long)]
        compress_output: Option<Option<CompressionType>>,
        /// Does not transform PotentiallPii data types
        #[structopt(long)]
        allow_potential_pii: bool,
        /// Does not transform Commercially sensitive data types
        #[structopt(long)]
        allow_commercially_sensitive: bool,
        /// Modifies the "Scramble" transformer to use an underscore for all replaced non-whitespace characters
        #[structopt(long)]
        scramble_blank: bool,
    },

    /// Creates a CSV file of PII or PotentialPII fields
    ToCsv {
        /// Path to write csv file to
        #[structopt(short, long, default_value = "./output.csv")]
        output_file: String,
        /// Path to the strategy.json file
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,
    },

    /// Checks the provided strategy file against a database to check that all fields are covered
    /// and valid
    CheckStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },

    /// Fixes errors in the strategy file
    FixStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },

    /// Generates a new skeleton strategy file from a db connection
    GenerateStrategies {
        #[structopt(short, long, default_value = "./strategy.json")]
        strategy_file: String,

        #[structopt(short, long, env = "DATABASE_URL")]
        db_url: String,
    },

    /// Uncompress a zstd sql dump to a file, or stdout if no file specified
    /// Does not currently work for gzip as tools to decompress that are more
    /// readily available
    Uncompress {
        /// Input file (*.sql.zst)
        #[structopt(short, long)]
        input_file: PathBuf,
        /// Output file, will write to standard output if not specified
        #[structopt(short, long)]
        output_file: Option<PathBuf>,
    },
}
