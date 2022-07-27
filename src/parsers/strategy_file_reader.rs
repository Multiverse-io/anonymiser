use crate::parsers::strategy_file_parser;
use crate::parsers::strategy_structs::*;
use itertools::sorted;
use itertools::Itertools;
use std::io::Write;

use serde_json;
use std::collections::HashMap;
use std::fs;

pub fn read(
    file_name: &str,
    transformer_overrides: TransformerOverrides,
) -> Result<Strategies, std::io::Error> {
    read_file(file_name)
        .map(|strategies| strategy_file_parser::parse(strategies, transformer_overrides))
}

pub fn sync_to_file(
    file_name: &str,
    missing_columns: Vec<SimpleColumn>,
    redundant_columns: Vec<SimpleColumn>,
) -> std::io::Result<()> {
    let current_file_contents = read_file(file_name).unwrap_or_else(|_| Vec::new());

    let file_contents_with_missing = add_missing(current_file_contents, &missing_columns);
    let new_file_contents = remove_redundant(file_contents_with_missing, &redundant_columns);

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
                new_strategies.push(new_table);
            }
        }
    }

    new_strategies.sort();

    return new_strategies;
}

fn remove_redundant(
    existing: Vec<StrategyInFile>,
    redundant_columns_to_remove: &Vec<SimpleColumn>,
) -> Vec<StrategyInFile> {
    let table_names = redundant_columns_to_remove
        .iter()
        .fold(HashMap::new(), |mut acc, column| {
            acc.entry(column.table_name.clone())
                .or_insert_with(|| vec![])
                .push(column.column_name.clone());
            return acc;
        });

    existing
        .into_iter()
        .filter_map(
            |strategy| match table_names.get(&strategy.table_name.clone()) {
                Some(columns_to_remove) => {
                    let new_columns: Vec<ColumnInFile> = strategy
                        .columns
                        .clone()
                        .into_iter()
                        .filter(|col| !columns_to_remove.contains(&col.name))
                        .collect();

                    if new_columns.len() > 0 {
                        let mut new_strategy = strategy.clone();
                        new_strategy.columns = new_columns;
                        Some(new_strategy)
                    } else {
                        None
                    }
                }
                None => Some(strategy),
            },
        )
        .collect()
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

fn read_file(file_name: &str) -> Result<Vec<StrategyInFile>, std::io::Error> {
    let result = fs::read_to_string(file_name).map(|file_contents| {
        let p: Vec<StrategyInFile> = serde_json::from_str(&file_contents).expect(&format!(
            "Invalid json found in strategy file at '{}'",
            file_name
        ));
        return p;
    });

    match result {
        Ok(_) => result,
        Err(ref err) => match err.kind() {
            std::io::ErrorKind::NotFound => result,
            _ => panic!("Unable to read strategy file at {}: {:?}", file_name, err),
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

    #[test]
    fn removes_redundant_columns() {
        let existing_columns = vec![
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

        let redundant_columns_to_remove = vec![
            SimpleColumn {
                table_name: "public.location".to_string(),
                column_name: "id".to_string(),
            },
            SimpleColumn {
                table_name: "public.location".to_string(),
                column_name: "post_code".to_string(),
            },
            SimpleColumn {
                table_name: "public.person".to_string(),
                column_name: "last_name".to_string(),
            },
        ];

        let result = remove_redundant(existing_columns, &redundant_columns_to_remove);

        let expected = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "".to_string(),
            columns: vec![ColumnInFile::new("id"), ColumnInFile::new("first_name")],
        }];

        assert_eq!(result, expected);
    }
}
