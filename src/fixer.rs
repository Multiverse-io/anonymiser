use crate::parsers::strategy_errors::StrategyFileError;
use crate::parsers::strategy_file;

pub fn can_fix(error: &StrategyFileError) -> bool {
    match error {
        StrategyFileError::ValidationError(validation_error) => {
            !validation_error.duplicate_columns.is_empty()
                || !validation_error.duplicate_tables.is_empty()
        }

        StrategyFileError::DbMismatchError(db_mismatch_error) => {
            !db_mismatch_error.missing_from_strategy_file.is_empty()
                || !db_mismatch_error.missing_from_db.is_empty()
        }
    }
}

pub fn fix_columns(strategy_file: &str, error: StrategyFileError) {
    match error {
        StrategyFileError::ValidationError(_validation_error) => {
            //TODO this
        }

        StrategyFileError::DbMismatchError(db_mismatch_error) => {
            let missing = db_mismatch_error.missing_from_strategy_file;

            let redundant = db_mismatch_error.missing_from_db;

            strategy_file::sync_to_file(strategy_file, missing, redundant)
                .expect("Unable to write to file :(");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_errors::{DbErrors, ValidationErrors};
    use crate::parsers::strategy_structs::SimpleColumn;

    #[test]
    fn cannot_fix_db_mismatch_error_if_no_missing_columns() {
        assert!(!can_fix(&StrategyFileError::DbMismatchError(DbErrors {
            missing_from_db: Vec::new(),
            missing_from_strategy_file: Vec::new(),
        })));
    }
    #[test]
    fn can_fix_db_mismatch_error_if_missing_from_db_and_strategy() {
        assert!(can_fix(&StrategyFileError::DbMismatchError(DbErrors {
            missing_from_db: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            missing_from_strategy_file: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
        })));
    }

    #[test]
    fn can_fix_db_mismatch_error_if_missing_from_db_only() {
        assert!(can_fix(&StrategyFileError::DbMismatchError(DbErrors {
            missing_from_db: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            missing_from_strategy_file: Vec::new(),
        })));
    }

    #[test]
    fn can_fix_db_mismatch_error_if_missing_from_strategy_file_only() {
        assert!(can_fix(&StrategyFileError::DbMismatchError(DbErrors {
            missing_from_db: Vec::new(),
            missing_from_strategy_file: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
        })));
    }
    #[test]
    fn cannot_fix_validation_error_if_no_errors() {
        assert!(!can_fix(&StrategyFileError::ValidationError(
            ValidationErrors {
                unknown_data_categories: Vec::new(),
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                duplicate_columns: Vec::new(),
                duplicate_tables: Vec::new(),
            }
        )));
    }

    #[test]
    fn cannot_fix_unknown_data_categories_error_transformer_types_or_unanoymised_pii() {
        let error = vec![SimpleColumn {
            column_name: "column".to_string(),
            table_name: "table".to_string(),
        }];
        assert!(!can_fix(&StrategyFileError::ValidationError(
            ValidationErrors {
                unknown_data_categories: error.clone(),
                error_transformer_types: error.clone(),
                unanonymised_pii: error,
                duplicate_columns: Vec::new(),
                duplicate_tables: Vec::new(),
            }
        )));
    }

    #[test]
    fn can_fix_duplicate_columns() {
        let error = vec![SimpleColumn {
            table_name: "table_name".to_string(),
            column_name: "column".to_string(),
        }];
        assert!(can_fix(&StrategyFileError::ValidationError(
            ValidationErrors {
                unknown_data_categories: Vec::new(),
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                duplicate_columns: error,
                duplicate_tables: Vec::new(),
            }
        )));
    }

    #[test]
    fn can_fix_duplicate_tables() {
        let error = vec!["table_name".to_string()];
        assert!(can_fix(&StrategyFileError::ValidationError(
            ValidationErrors {
                unknown_data_categories: Vec::new(),
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                duplicate_columns: Vec::new(),
                duplicate_tables: error,
            }
        )));
    }
}
