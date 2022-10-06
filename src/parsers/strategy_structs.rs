use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub struct ColumnInFile {
    pub data_category: DataCategory,
    pub description: String,
    pub name: String,

    pub transformer: Transformer,
}

impl ColumnInFile {
    //TODO why is this no longer used?!
    pub fn new(column_name: &str) -> Self {
        ColumnInFile {
            data_category: DataCategory::Unknown,
            description: "".to_string(),
            name: column_name.to_string(),
            transformer: Transformer {
                name: TransformerType::Error,
                args: None,
            },
        }
    }
}

impl Ord for ColumnInFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for ColumnInFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ColumnInFile {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub struct StrategyInFile {
    pub table_name: String,
    pub description: String,
    pub columns: Vec<ColumnInFile>,
}

impl Ord for StrategyInFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.table_name.cmp(&other.table_name)
    }
}

impl PartialOrd for StrategyInFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for StrategyInFile {
    fn eq(&self, other: &Self) -> bool {
        self.table_name == other.table_name && self.columns == other.columns
    }
}
#[derive(Debug)]
pub enum StrategyFileError {
    ValidationError(StrategyFileErrors),
    DbMismatchError(StrategyFileDbValidationErrors),
}

impl From<StrategyFileErrors> for StrategyFileError {
    fn from(err: StrategyFileErrors) -> Self {
        StrategyFileError::ValidationError(err)
    }
}

impl From<StrategyFileDbValidationErrors> for StrategyFileError {
    fn from(err: StrategyFileDbValidationErrors) -> Self {
        StrategyFileError::DbMismatchError(err)
    }
}

#[derive(Debug)]
pub struct StrategyFileDbValidationErrors {
    pub missing_from_strategy_file: Vec<SimpleColumn>,
    pub missing_from_db: Vec<SimpleColumn>,
}
impl StrategyFileDbValidationErrors {
    pub fn is_empty(to_check: &StrategyFileDbValidationErrors) -> bool {
        to_check.missing_from_strategy_file.is_empty() && to_check.missing_from_db.is_empty()
    }
}

#[derive(Debug)]
pub struct StrategyFileErrors {
    pub unknown_data_categories: Vec<SimpleColumn>,
    pub error_transformer_types: Vec<SimpleColumn>,
    pub unanonymised_pii: Vec<SimpleColumn>,
    pub duplicate_columns: Vec<(String, String)>,
    pub duplicate_tables: Vec<String>,
}

impl StrategyFileErrors {
    pub fn new() -> Self {
        StrategyFileErrors {
            unknown_data_categories: Vec::new(),
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            duplicate_columns: Vec::new(),
            duplicate_tables: Vec::new(),
        }
    }
    pub fn is_empty(to_check: &StrategyFileErrors) -> bool {
        to_check.unknown_data_categories.is_empty()
            && to_check.error_transformer_types.is_empty()
            && to_check.unanonymised_pii.is_empty()
            && to_check.duplicate_columns.is_empty()
            && to_check.duplicate_tables.is_empty()
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct SimpleColumn {
    pub table_name: String,
    pub column_name: String,
}
impl Ord for SimpleColumn {
    fn cmp(&self, other: &Self) -> Ordering {
        format!("{}{}", self.table_name, self.column_name)
            .cmp(&format!("{}{}", other.table_name, other.column_name))
    }
}

impl PartialOrd for SimpleColumn {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnInfo {
    pub data_category: DataCategory,
    pub name: String,
    pub transformer: Transformer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DataCategory {
    CommerciallySensitive,
    General,
    PotentialPii,
    Pii,
    Security,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransformerType {
    EmptyJson,
    Error,
    FakeBase16String,
    FakeBase32String,
    FakeCity,
    FakeCompanyName,
    FakeEmail,
    FakeFirstName,
    FakeFullAddress,
    FakeFullName,
    FakeIPv4,
    FakeLastName,
    FakeNationalIdentityNumber,
    FakePhoneNumber,
    FakePostCode,
    FakeState,
    FakeStreetAddress,
    FakeUsername,
    FakeUUID,
    Fixed,
    Identity,
    ObfuscateDay,
    Scramble,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Transformer {
    pub name: TransformerType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, String>>,
}

pub struct TransformerOverrides {
    pub allow_potential_pii: bool,
    pub allow_commercially_sensitive: bool,
}

impl TransformerOverrides {
    pub fn none() -> Self {
        Self {
            allow_potential_pii: false,
            allow_commercially_sensitive: false,
        }
    }
}
