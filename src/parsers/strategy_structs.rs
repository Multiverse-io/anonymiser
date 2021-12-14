use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct ColumnInFile {
    pub name: String,
    pub transformer: Transformer,
}
#[derive(Serialize, Deserialize)]
pub struct StrategyInFile {
    pub table_name: String,
    pub schema: String,
    pub columns: Vec<ColumnInFile>,
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

pub type Strategies = HashMap<String, HashMap<String, Transformer>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transformer {
    pub name: TransformerType,
    pub args: Option<HashMap<String, String>>,
}
