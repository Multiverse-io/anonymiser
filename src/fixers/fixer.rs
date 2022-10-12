use crate::fixers::{db_mismatch, validation};
use crate::parsers::strategy_errors::StrategyFileError;
use crate::parsers::strategy_file;

pub fn can_fix(error: &StrategyFileError) -> bool {
    match error {
        StrategyFileError::ValidationError(validation_error) => {
            !validation_error.duplicate_columns.is_empty()
        }
        StrategyFileError::DbMismatchError(db_mismatch_error) => {
            !db_mismatch_error.missing_from_strategy_file.is_empty()
                || !db_mismatch_error.missing_from_db.is_empty()
        }
    }
}

pub enum SortResult {
    Sorted,
    NoChange,
}

pub fn just_sort(strategy_file: &str) -> SortResult {
    let initial_hash = sha256_digest(strategy_file);
    let current_file_contents = strategy_file::read(strategy_file).unwrap_or_else(|_| Vec::new());
    strategy_file::write(strategy_file, current_file_contents).expect("Unable to write to file :(");
    let post_sort_hash = sha256_digest(strategy_file);
    if initial_hash == post_sort_hash {
        SortResult::NoChange
    } else {
        SortResult::Sorted
    }
}

pub fn fix_columns(strategy_file: &str, error: StrategyFileError) {
    let current_file_contents = strategy_file::read(strategy_file).unwrap_or_else(|_| Vec::new());
    match error {
        StrategyFileError::ValidationError(validation_error) => {
            let new_file_contents = validation::fix(current_file_contents, validation_error);

            strategy_file::write(strategy_file, new_file_contents)
                .expect("Unable to write to file :(");
        }
        StrategyFileError::DbMismatchError(db_mismatch_error) => {
            let new_file_contents = db_mismatch::fix(current_file_contents, db_mismatch_error);

            strategy_file::write(strategy_file, new_file_contents)
                .expect("Unable to write to file :(");
        }
    }
}
fn sha256_digest(strategy_file: &str) -> String {
    let bytes = std::fs::read(strategy_file).unwrap();
    sha256::digest_bytes(&bytes)
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
    fn cannot_currently_fix_duplicate_tables() {
        let error = vec!["table_name".to_string()];
        assert!(!can_fix(&StrategyFileError::ValidationError(
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
