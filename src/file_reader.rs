use crate::parsers::row_parser;
use crate::parsers::state::State;
use crate::parsers::strategies::Strategies;
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

    let file_reader = File::open(&input_file_path)
        .unwrap_or_else(|_| panic!("Input file '{}' does not exist", input_file_path));

    let mut reader = BufReader::new(file_reader);
    let mut line = String::new();

    let mut row_parser_state = State::new();

    loop {
        match reader.read_line(&mut line) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }

                line = line.to_string();
                let transformed_row = row_parser::parse(&line, &mut row_parser_state, strategies);
                file_writer.write_all(transformed_row.as_bytes())?;
                line.clear();
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn can_read() {
        let input_file = "test_files/dump_file.sql".to_string();
        let output_file = "test_files/file_reader_test_results.sql".to_string();
        let mut strategies = Strategies::new();
        strategies.insert(
            "public.orders".to_string(),
            HashMap::from([
                ("id".to_string(), column_info()),
                ("user_id".to_string(), column_info()),
                ("product_id".to_string(), column_info()),
            ]),
        );
        strategies.insert(
            "public.products".to_string(),
            HashMap::from([
                ("id".to_string(), column_info()),
                ("description".to_string(), column_info()),
                ("price".to_string(), column_info()),
            ]),
        );

        strategies.insert(
            "public.users".to_string(),
            HashMap::from([
                ("id".to_string(), column_info()),
                ("email".to_string(), column_info()),
                ("password".to_string(), column_info()),
                ("last_login".to_string(), column_info()),
                ("inserted_at".to_string(), column_info()),
                ("updated_at".to_string(), column_info()),
                ("first_name".to_string(), column_info()),
                ("last_name".to_string(), column_info()),
                ("deactivated".to_string(), column_info()),
                ("phone_number".to_string(), column_info()),
            ]),
        );

        assert!(read(input_file.clone(), output_file.clone(), &strategies).is_ok());

        let original =
            fs::read_to_string(&input_file).expect("Something went wrong reading the file");

        let processed =
            fs::read_to_string(&output_file).expect("Something went wrong reading the file");

        assert_eq!(original, processed);
    }

    fn column_info() -> ColumnInfo {
        ColumnInfo {
            data_category: DataCategory::General,
            transformer: Transformer {
                name: TransformerType::Identity,
                args: None,
            },
        }
    }
}
