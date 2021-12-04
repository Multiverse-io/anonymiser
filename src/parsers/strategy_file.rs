use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use std::fs;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TransformerType {
    EmptyJson,
    Error,
    FakeCity,
    FakeCompanyName,
    FakeEmail,
    FakeFirstName,
    FakeFullAddress,
    FakeFullName,
    FakeIPv4,
    FakeLastName,
    FakePostCode,
    FakeState,
    FakeStreetAddress,
    FakeUUID,
    Fixed,
    Identity,
    ObfuscateDay,
    Redact,
    Scramble,
    Test,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transformer {
    pub name: TransformerType,
    pub args: Option<HashMap<String, String>>,
}
#[derive(Serialize, Deserialize)]
struct ColumnInFile {
    name: String,
    transformer: Transformer,
}
#[derive(Serialize, Deserialize)]
struct StrategyInFile {
    table_name: String,
    schema: String,
    columns: Vec<ColumnInFile>,
}

pub fn parse(file_name: &str) -> HashMap<String, HashMap<String, Transformer>> {
    match read_file(file_name) {
        Ok(strategies) => transform_file_strategies(strategies),
        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

fn transform_file_strategies(
    strategies: Vec<StrategyInFile>,
) -> HashMap<String, HashMap<String, Transformer>> {
    let mut transformed_strategies: HashMap<String, HashMap<String, Transformer>> = HashMap::new();
    //TODO If all columns are none, lets not do any transforming?
    for strategy in strategies {
        let columns = strategy
            .columns
            .into_iter()
            .map(|column| (column.name, column.transformer))
            .collect();

        transformed_strategies.insert(
            format!("{}.{}", strategy.schema, strategy.table_name),
            columns,
        );
    }
    return transformed_strategies;
}

fn read_file(file_name: &str) -> Result<Vec<StrategyInFile>> {
    match fs::read_to_string(file_name) {
        Ok(file_contents) => {
            let p: Vec<StrategyInFile> = serde_json::from_str(&file_contents)?;
            return Ok(p);
        }

        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //TODO proper tests here
    #[test]
    fn it_works() {
        let x = parse("new_mappings.json");
        print!("{:?}", x);
    }
}
