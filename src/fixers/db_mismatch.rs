use crate::parsers::strategy_errors::DbErrors;
use crate::parsers::strategy_structs::{ColumnInFile, SimpleColumn, StrategyInFile};
use std::collections::HashMap;

pub fn fix(
    current_file_contents: Vec<StrategyInFile>,
    db_mismatch_error: DbErrors,
) -> Vec<StrategyInFile> {
    let missing_columns = db_mismatch_error.missing_from_strategy_file;
    let redundant_columns = db_mismatch_error.missing_from_db;
    let file_contents_with_missing = add_missing(current_file_contents, &missing_columns);
    remove_redundant(file_contents_with_missing, &redundant_columns)
}

fn add_missing(current: Vec<StrategyInFile>, missing: &[SimpleColumn]) -> Vec<StrategyInFile> {
    let missing_columns_by_table = missing.iter().fold(HashMap::new(), |mut acc, column| {
        acc.entry(column.table_name.clone())
            .or_insert_with(Vec::new)
            .push(column.column_name.clone());
        acc
    });

    let mut new_strategies = current;

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

    new_strategies
}

fn remove_redundant(
    existing: Vec<StrategyInFile>,
    redundant_columns_to_remove: &[SimpleColumn],
) -> Vec<StrategyInFile> {
    let table_names = redundant_columns_to_remove
        .iter()
        .fold(HashMap::new(), |mut acc, column| {
            acc.entry(column.table_name.clone())
                .or_insert_with(Vec::new)
                .push(column.column_name.clone());
            acc
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

                    if new_columns.is_empty() {
                        None
                    } else {
                        let mut new_strategy = strategy;
                        new_strategy.columns = new_columns;
                        Some(new_strategy)
                    }
                }
                None => Some(strategy),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_missing_columns() {
        let current = vec![StrategyInFile {
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

        let result = add_missing(current, &missing);

        let expected = vec![
            StrategyInFile {
                table_name: "public.person".to_string(),
                description: "".to_string(),
                columns: vec![
                    ColumnInFile::new("id"),
                    ColumnInFile::new("first_name"),
                    ColumnInFile::new("last_name"),
                ],
            },
            StrategyInFile {
                table_name: "public.location".to_string(),
                description: "".to_string(),
                columns: vec![ColumnInFile::new("id"), ColumnInFile::new("post_code")],
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
