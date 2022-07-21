use crate::parsers::strategy_file_parser;
use crate::parsers::strategy_structs::*;
use itertools::sorted;
use itertools::Itertools;
use std::io::Write;

use serde_json;
use std::collections::HashMap;
use std::fs;

pub fn read(file_name: &str, transformer_overrides: TransformerOverrides) -> Strategies {
    match read_file(file_name) {
        Ok(strategies) => strategy_file_parser::parse(strategies, transformer_overrides),
        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

pub fn append_to_file(file_name: &str, missing_columns: Vec<SimpleColumn>) -> std::io::Result<()> {
    let current_file_contents = read_file(file_name).unwrap();

    let new_file_contents = add_missing(current_file_contents, &missing_columns);

    let file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file_name)?;

    serde_json::to_writer_pretty(file, &new_file_contents)?;

    Ok(())
}

fn add_missing(present: Vec<StrategyInFile>, missing: &Vec<SimpleColumn>) -> Vec<StrategyInFile> {
    let missing_columns_by_table = missing.iter().fold(HashMap::new(), |mut acc, column| {
        acc.entry(column.table_name.clone())
            .or_insert_with(|| vec![])
            .push(column.column_name.clone());
        return acc;
    });

    let mut new_strategies = present;

    for (table, missing_columns) in missing_columns_by_table {
        match new_strategies.iter().position(|c| c.table_name == table) {
            Some(position) => {
                let existing_table = new_strategies.get_mut(position).unwrap();
                for column in missing_columns {
                    existing_table.columns.push(ColumnInFile::new(&column));
                }
                existing_table.columns.sort();
            }
            None => {
                let mut new_table = StrategyInFile {
                    table_name: table.clone(),
                    description: "".to_string(),
                    columns: vec![],
                };
                for column in missing_columns {
                    new_table.columns.push(ColumnInFile::new(&column));
                }
                new_table.columns.sort();
                new_strategies.push(new_table);
            }
        }
    }

    new_strategies.sort();

    return new_strategies;
}

pub fn to_csv(strategy_file: &str, csv_output_file: &str) -> std::io::Result<()> {
    let strategies = read_file(strategy_file)?;
    let p: Vec<String> = strategies
        .iter()
        .flat_map(|strategy| {
            strategy.columns.iter().filter_map(|column| {
                if column.data_type == DataType::Pii || column.data_type == DataType::PotentialPii {
                    return Some(format!(
                        "{}, {}, {}",
                        strategy.table_name, column.name, column.description
                    ));
                } else {
                    return None;
                }
            })
        })
        .collect::<Vec<String>>();
    let to_write = format!(
        "{}\n{}",
        "table name, column name, description",
        sorted(p).join("\n")
    );

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(csv_output_file)?;
    file.write_all(to_write.as_bytes()).unwrap();

    return Ok(());
}

fn read_file(file_name: &str) -> serde_json::Result<Vec<StrategyInFile>> {
    match fs::read_to_string(file_name) {
        Ok(file_contents) => {
            let p: Vec<StrategyInFile> = serde_json::from_str(&file_contents)?;
            return Ok(p);
        }

        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Ok(vec![]),
            _ => panic!("Unable to read strategy file at {}: {:?}", file_name, e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_missing_columns() {
        let present = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "".to_string(),
            columns: vec![ColumnInFile::new("id"), ColumnInFile::new("first_name")],
        }];

        let missing = vec![
            SimpleColumn {
                table_name: "public.person".to_string(),
                column_name: "id".to_string(),
            },
            SimpleColumn {
                table_name: "public.person".to_string(),
                column_name: "first_name".to_string(),
            },
            SimpleColumn {
                table_name: "public.person".to_string(),
                column_name: "last_name".to_string(),
            },
            SimpleColumn {
                table_name: "public.location".to_string(),
                column_name: "id".to_string(),
            },
            SimpleColumn {
                table_name: "public.location".to_string(),
                column_name: "post_code".to_string(),
            },
        ];

        let result = add_missing(present, &missing);

        let expected = vec![
            StrategyInFile {
                table_name: "public.location".to_string(),
                description: "".to_string(),
                columns: vec![ColumnInFile::new("id"), ColumnInFile::new("post_code")],
            },
            StrategyInFile {
                table_name: "public.person".to_string(),
                description: "".to_string(),
                columns: vec![
                    ColumnInFile::new("id"),
                    ColumnInFile::new("first_name"),
                    ColumnInFile::new("last_name"),
                ],
            },
        ];

        assert_eq!(result, expected);
    }
}
