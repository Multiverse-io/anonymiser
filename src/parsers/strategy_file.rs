use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize)]
struct ColumnInFile {
    name: String,
    transformer: String,
}
#[derive(Serialize, Deserialize)]
struct StrategyInFile {
    table_name: String,
    schema: String,
    columns: Vec<ColumnInFile>,
}

pub fn parse(file_name: &str) -> HashMap<String, Vec<String>> {
    match read_file(file_name) {
        Ok(strategies) => transform_file_strategies(strategies),
        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

fn transform_file_strategies(strategies: Vec<StrategyInFile>) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for strategy in strategies {
        let columns = strategy
            .columns
            .into_iter()
            .map(|column| column.transformer)
            .collect();
        map.insert(strategy.table_name, columns);
    }
    return map;
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

    #[test]
    fn it_works() {
        let x = parse("new_mappings.json");
        print!("{:?}", x);
    }
}
