use crate::parsers::strategy_structs::*;
use itertools::Itertools;
use std::fmt;
use std::fmt::Write;

#[derive(Debug)]
pub enum StrategyFileError {
    ValidationError(Box<ValidationErrors>),
    DbMismatchError(Box<DbErrors>),
}
impl fmt::Display for StrategyFileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StrategyFileError::ValidationError(error) => write!(f, "{}", error),
            StrategyFileError::DbMismatchError(error) => write!(f, "{}", error),
        }
    }
}

impl From<ValidationErrors> for StrategyFileError {
    fn from(err: ValidationErrors) -> Self {
        StrategyFileError::ValidationError(Box::new(err))
    }
}

impl From<DbErrors> for StrategyFileError {
    fn from(err: DbErrors) -> Self {
        StrategyFileError::DbMismatchError(Box::new(err))
    }
}

#[derive(Debug, PartialEq)]
pub struct DbErrors {
    pub missing_from_strategy_file: Vec<SimpleColumn>,
    pub missing_from_db: Vec<SimpleColumn>,
}
impl DbErrors {
    pub fn is_empty(to_check: &DbErrors) -> bool {
        to_check.missing_from_strategy_file.is_empty() && to_check.missing_from_db.is_empty()
    }
}

impl fmt::Display for DbErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = "".to_string();
        if !self.missing_from_db.is_empty() {
            let missing_list = column_to_message(&self.missing_from_db);
            write!(
                message,
                "Some fields are in the strategy file but not the database!\n\t{}\n",
                missing_list
            )
            .unwrap()
        }

        if !self.missing_from_strategy_file.is_empty() {
            let missing_list = column_to_message(&self.missing_from_strategy_file);
            write!(
                message,
                "Some fields are missing from strategy file\n\t{}\n",
                missing_list
            )
            .unwrap()
        }
        write!(f, "{}", message)
    }
}

#[derive(Debug)]
pub struct ValidationErrors {
    pub unknown_data_categories: Vec<SimpleColumn>,
    pub error_transformer_types: Vec<SimpleColumn>,
    pub unanonymised_pii: Vec<SimpleColumn>,
    pub duplicate_columns: Vec<SimpleColumn>,
    pub duplicate_tables: Vec<String>,
    pub deterministic_without_id: Vec<SimpleColumn>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = "".to_string();
        if !self.unanonymised_pii.is_empty() {
            let missing_list = column_to_message(&self.unanonymised_pii);
            write!(message,
                 "Some fields in strategy file are tagged as being PII but do not have anonymising transformers set\n\t{}\nPlease add valid transformers!\n\n",
                missing_list
            ).unwrap()
        }

        if !self.error_transformer_types.is_empty() {
            let missing_list = column_to_message(&self.error_transformer_types);
            write!(message, "Some fields in strategy file still have 'Error' transformer types\n\t{}\nPlease add valid transformers!\n\n",
                missing_list
            ).unwrap()
        }

        if !self.unknown_data_categories.is_empty() {
            let missing_list = column_to_message(&self.unknown_data_categories);
            write!(message,
                "Some fields in strategy file still have 'Unknown' data types\n\t{}\nPlease add valid data types!\n\n",
                missing_list
            ).unwrap()
        }

        if !self.duplicate_columns.is_empty() {
            let missing_list = column_to_message(&self.unknown_data_categories);
            write!(
                message,
                "Duplicate columns definitions found in strategy file\n\t{}\n\n",
                missing_list
            )
            .unwrap()
        }

        if !self.duplicate_tables.is_empty() {
            write!(
                message,
                "Duplicate table definitions found in strategy file\n\t{}\n\n",
                self.duplicate_tables.iter().join("\n\t")
            )
            .unwrap()
        }

        write!(f, "{}", message)
    }
}
fn column_to_message(column: &[SimpleColumn]) -> String {
    return column
        .iter()
        .map(|c| format!("{} => {}", &c.table_name, &c.column_name))
        .sorted()
        .join("\n\t");
}

impl ValidationErrors {
    pub fn new() -> Self {
        ValidationErrors {
            unknown_data_categories: Vec::new(),
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            duplicate_columns: Vec::new(),
            duplicate_tables: Vec::new(),
            deterministic_without_id: Vec::new(),
        }
    }
    pub fn is_empty(to_check: &ValidationErrors) -> bool {
        to_check.unknown_data_categories.is_empty()
            && to_check.error_transformer_types.is_empty()
            && to_check.unanonymised_pii.is_empty()
            && to_check.duplicate_columns.is_empty()
            && to_check.duplicate_tables.is_empty()
            && to_check.deterministic_without_id.is_empty()
    }
}
