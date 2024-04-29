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
    FakeEmailOrPhone,
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
    ScrambleBlank,
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
    pub scramble_blank: bool,
}

impl TransformerOverrides {
    pub fn none() -> Self {
        Self {
            allow_potential_pii: false,
            allow_commercially_sensitive: false,
            scramble_blank: false,
        }
    }
}

impl Default for TransformerOverrides {
    fn default() -> Self {
        Self::none()
    }
}
