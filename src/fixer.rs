use crate::parsers::strategy_file;
use crate::parsers::strategy_structs::MissingColumns;

pub fn can_fix(missing_columns: &MissingColumns) -> bool {
    !missing_columns.missing_from_strategy_file.is_empty()
        || !missing_columns.missing_from_db.is_empty()
}

pub fn fix_columns(strategy_file: &str, missing_columns: MissingColumns) {
    let missing = missing_columns.missing_from_strategy_file;

    let redundant = missing_columns.missing_from_db;

    strategy_file::sync_to_file(strategy_file, missing, redundant)
        .expect("Unable to write to file :(");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::SimpleColumn;

    #[test]
    fn cannot_fix_if_no_missing_columns() {
        assert_eq!(
            false,
            can_fix(&MissingColumns {
                missing_from_db: Vec::new(),
                missing_from_strategy_file: Vec::new(),
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                unknown_data_categories: Vec::new()
            })
        );
    }
    #[test]
    fn can_fix_if_missing_from_db_and_strategy() {
        assert_eq!(
            true,
            can_fix(&MissingColumns {
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
            })
        );
    }

    #[test]
    fn can_fix_if_missing_from_db_only() {
        assert_eq!(
            true,
            can_fix(&MissingColumns {
                missing_from_db: vec![SimpleColumn {
                    column_name: "column".to_string(),
                    table_name: "table".to_string()
                }],
                missing_from_strategy_file: Vec::new(),
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                unknown_data_categories: Vec::new()
            })
        );
    }

    #[test]
    fn can_fix_if_missing_from_strategy_file_only() {
        assert_eq!(
            true,
            can_fix(&MissingColumns {
                missing_from_db: Vec::new(),
                missing_from_strategy_file: vec![SimpleColumn {
                    column_name: "column".to_string(),
                    table_name: "table".to_string()
                }],
                error_transformer_types: Vec::new(),
                unanonymised_pii: Vec::new(),
                unknown_data_categories: Vec::new()
            })
        );
    }
}
