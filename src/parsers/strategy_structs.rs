use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct ColumnInFile {
    pub data_type: DataType,
    pub description: String,
    pub name: String,
    pub transformer: Transformer,
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

#[derive(Debug, Eq, Serialize, Deserialize)]
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
        self.table_name == other.table_name
    }
}

#[derive(Debug)]
pub struct MissingColumns {
    pub missing_from_strategy_file: Option<Vec<SimpleColumn>>,
    pub missing_from_db: Option<Vec<SimpleColumn>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct SimpleColumn {
    pub table_name: String,
    pub column_name: String,
}

pub struct ColumnInfo {
    pub data_type: DataType,
    pub transformer: Transformer,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    General,
    Unknown,
}

pub type Strategies = HashMap<String, HashMap<String, ColumnInfo>>;

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
    Redact,
    Scramble,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Transformer {
    pub name: TransformerType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, String>>,
}
