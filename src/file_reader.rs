use crate::parsers::row_parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;

pub fn read(strategies: &HashMap<String, HashMap<String, String>>) -> Result<(), std::io::Error> {
    let output_file = File::create("poem.sql").unwrap();
    let mut file_writer = BufWriter::new(output_file);

    let file_reader = File::open("clear_text_dump_big.sql")?;
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
                let to_write = [transformed_row.as_bytes(), "\n".as_bytes()].concat();
                file_writer.write_all(&to_write);
                line.clear();
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    return Ok(());
}
