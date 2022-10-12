use crate::parsers::strategy_errors::ValidationErrors;
use crate::parsers::strategy_structs::StrategyInFile;
use std::collections::HashMap;

pub fn fix(
    mut current_file_contents: Vec<StrategyInFile>,
    validation_errors: ValidationErrors,
) -> Vec<StrategyInFile> {
    let mut table_to_duplicate_columns = HashMap::new();
    for d in validation_errors.duplicate_columns {
        table_to_duplicate_columns
            .entry(d.table_name)
            .or_insert_with(Vec::new)
            .push(d.column_name)
    }
    current_file_contents.iter_mut().for_each(|s| {
        let duplicate_columns_for_table = &table_to_duplicate_columns[&s.table_name];
        if !duplicate_columns_for_table.is_empty() {
            for duplicate_column_name in duplicate_columns_for_table {
                let mut already_have_one = false;
                s.columns.retain(|column| {
                    if duplicate_column_name == &column.name {
                        if !already_have_one {
                            already_have_one = true;
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                });
            }
        }
    });

    current_file_contents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_errors::ValidationErrors;
    use crate::parsers::strategy_structs::{ColumnInFile, SimpleColumn, StrategyInFile};

    #[test]
    fn can_remove_duplicate_columns() {
        let current_file_contents = vec![StrategyInFile::builder()
            .with_table_name("Tabby")
            .with_column(ColumnInFile::builder().with_name("id").build())
            .with_column(ColumnInFile::builder().with_name("name").build())
            .with_column(ColumnInFile::builder().with_name("id").build())
            .with_column(ColumnInFile::builder().with_name("name").build())
            .build()];

        let mut errors = ValidationErrors::new();
        errors.duplicate_columns = vec![
            SimpleColumn {
                table_name: "Tabby".to_string(),
                column_name: "id".to_string(),
            },
            SimpleColumn {
                table_name: "Tabby".to_string(),
                column_name: "name".to_string(),
            },
        ];

        let expected_file_contents = vec![StrategyInFile::builder()
            .with_table_name("Tabby")
            .with_column(ColumnInFile::builder().with_name("id").build())
            .with_column(ColumnInFile::builder().with_name("name").build())
            .build()];

        let new_file_contents = fix(current_file_contents, errors);

        assert_eq!(new_file_contents, expected_file_contents);
    }
}
