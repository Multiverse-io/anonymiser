use crate::parsers::strategy_file;
use crate::parsers::strategy_structs::StrategyFileError;

pub fn can_fix(error: &StrategyFileError) -> bool {
    match error {
        StrategyFileError::ValidationError(validation_error) => false, //TODO this
        StrategyFileError::DbMismatchError(db_mismatch_error) => {
            !db_mismatch_error.missing_from_strategy_file.is_empty()
                || !db_mismatch_error.missing_from_db.is_empty()
        }
    }
}

pub fn fix_columns(strategy_file: &str, errors: StrategyFileError) {
    //TODO this
    //let missing = strategy_errors.missing_from_strategy_file;

    //let redundant = strategy_errors.missing_from_db;

    //strategy_file::sync_to_file(strategy_file, missing, redundant)
    //    .expect("Unable to write to file :(");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::SimpleColumn;

    #[test]
    fn cannot_fix_if_no_missing_columns() {
        assert!(!can_fix(&StrategyFileErrors {
            missing_from_db: Vec::new(),
            missing_from_strategy_file: Vec::new(),
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            unknown_data_categories: Vec::new()
        }));
    }
    #[test]
    fn can_fix_if_missing_from_db_and_strategy() {
        assert!(can_fix(&StrategyFileErrors {
            missing_from_db: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            missing_from_strategy_file: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            unknown_data_categories: Vec::new()
        }));
    }

    #[test]
    fn can_fix_if_missing_from_db_only() {
        assert!(can_fix(&StrategyFileErrors {
            missing_from_db: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            missing_from_strategy_file: Vec::new(),
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            unknown_data_categories: Vec::new()
        }));
    }

    #[test]
    fn can_fix_if_missing_from_strategy_file_only() {
        assert!(can_fix(&StrategyFileErrors {
            missing_from_db: Vec::new(),
            missing_from_strategy_file: vec![SimpleColumn {
                column_name: "column".to_string(),
                table_name: "table".to_string()
            }],
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            unknown_data_categories: Vec::new()
        }));
    }
}
