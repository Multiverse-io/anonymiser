use crate::parsers::strategy_structs::*;
use itertools::sorted;
use itertools::Itertools;
use std::io::Write;

use serde_json;
use std::collections::HashMap;
use std::fs;

pub fn parse(file_name: &str, allow_potential_pii: bool) -> Strategies {
    match read_file(file_name) {
        Ok(strategies) => transform_file_strategies(strategies, allow_potential_pii),
        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

pub fn append_to_file(file_name: &str, missing_columns: Vec<SimpleColumn>) -> std::io::Result<()> {
    let missing_columns_by_table =
        missing_columns
            .iter()
            .fold(HashMap::new(), |mut acc, column| {
                acc.entry(column.table_name.clone())
                    .or_insert_with(|| vec![])
                    .push(column.column_name.clone());
                return acc;
            });

    let mut current_file_contents = read_file(file_name).unwrap();

    for (table, missing_columns) in missing_columns_by_table {
        match current_file_contents
            .iter()
            .position(|c| c.table_name == table)
        {
            Some(position) => {
                let existing_table = current_file_contents.get_mut(position).unwrap();
                for column in missing_columns {
                    existing_table.columns.push(ColumnInFile {
                        data_type: DataType::Unknown,
                        description: "".to_string(),
                        name: column,
                        transformer: Transformer {
                            name: TransformerType::Error,
                            args: None,
                        },
                    });
                    existing_table.columns.sort();
                }
            }
            //TODO deal with whole missing table VV
            None => {
                panic!("We dont deal with the table not existing yet! we can patch columns but not tables!");
            }
        }
    }
    current_file_contents.sort();

    let file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_name)?;
    serde_json::to_writer_pretty(file, &current_file_contents)?;
    return Ok(());
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

fn transformer(column: ColumnInFile, allow_potential_pii: bool) -> Transformer {
    if allow_potential_pii && column.data_type == DataType::PotentialPii {
        return Transformer {
            name: TransformerType::Identity,
            args: None,
        };
    } else {
        return column.transformer;
    };
}
fn transform_file_strategies(
    strategies: Vec<StrategyInFile>,
    allow_potential_pii: bool,
) -> HashMap<String, HashMap<String, ColumnInfo>> {
    let mut transformed_strategies: HashMap<String, HashMap<String, ColumnInfo>> = HashMap::new();
    //TODO If all columns are none, lets not do any transforming?
    for strategy in strategies {
        let columns = strategy
            .columns
            .into_iter()
            .map(|column| {
                return (
                    column.name.clone(),
                    ColumnInfo {
                        data_type: column.data_type.clone(),
                        transformer: transformer(column, allow_potential_pii),
                    },
                );
            })
            .collect();

        transformed_strategies.insert(strategy.table_name, columns);
    }

    return transformed_strategies;
}

fn read_file(file_name: &str) -> serde_json::Result<Vec<StrategyInFile>> {
    match fs::read_to_string(file_name) {
        Ok(file_contents) => {
            let p: Vec<StrategyInFile> = serde_json::from_str(&file_contents)?;
            return Ok(p);
        }

        Err(error) => panic!("Unable to read strategy file at {}: {:?}", file_name, error),
    }
}
