use crate::parsers::row_parser;
use crate::parsers::strategy_structs::Strategies;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;

pub fn read(
    input_file_path: String,
    output_file_path: String,
    strategies: &Strategies,
) -> Result<(), std::io::Error> {
    let output_file = File::create(output_file_path).unwrap();
    let mut file_writer = BufWriter::new(output_file);

    let file_reader = File::open(input_file_path)?;
    let mut reader = BufReader::new(file_reader);
    let mut line = String::new();

    let mut row_parser_state = row_parser::initial_state();

    loop {
        match reader.read_line(&mut line) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }

                let transformed_row = row_parser::parse(&line, &mut row_parser_state, strategies);
                file_writer.write_all(transformed_row.as_bytes())?;
                line.clear();
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    return Ok(());
}
