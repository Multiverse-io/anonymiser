use crate::parsers::strategy_errors::ValidationErrors;
use crate::parsers::strategy_structs::{ColumnInFile, SimpleColumn, StrategyInFile};
use std::collections::HashMap;

pub fn fix(
    mut current_file_contents: Vec<StrategyInFile>,
    validation_errors: ValidationErrors,
) -> Vec<StrategyInFile> {
    //TODO this VV

    current_file_contents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_errors::{DbErrors, ValidationErrors};
    use crate::parsers::strategy_structs::SimpleColumn;

    #[test]
    fn can_remove_duplicate_columns() {
        let current_file_contents = vec![StrategyInFile::builder()
            .with_table_name("Tabby")
            .with_column(ColumnInFile::builder().with_name("id").build())
            .with_column(ColumnInFile::builder().with_name("id").build())
            .build()];

        let mut errors = ValidationErrors::new();
        errors.duplicate_columns = vec![SimpleColumn {
            table_name: "Tabby".to_string(),
            column_name: "id".to_string(),
        }];

        let expected_file_contents = vec![StrategyInFile::builder()
            .with_table_name("Tabby")
            .with_column(ColumnInFile::builder().with_name("id").build())
            .build()];

        let new_file_contents = fix(current_file_contents, errors);

        assert_eq!(new_file_contents, expected_file_contents);
    }
}
