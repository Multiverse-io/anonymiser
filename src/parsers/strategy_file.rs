use crate::parsers::transformer::Transformer;
use itertools::Itertools;
use postgres::GenericClient;
use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

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

#[derive(Debug)]
pub struct MissingColumns {
    missing_from_strategy_file: Option<Vec<SimpleColumn>>,
    missing_from_db: Option<Vec<SimpleColumn>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct SimpleColumn {
    table_name: String,
    column_name: String,
}

type Strategies = HashMap<String, HashMap<String, Transformer>>;

pub fn parse(file_name: String) -> Strategies {
    match read_file(file_name) {
        Ok(strategies) => transform_file_strategies(strategies),
        Err(error) => panic!("Unable to read strategy file: {:?}", error),
    }
}

pub fn generate<T>(strategies: Strategies, connection: &mut T) -> Result<(), MissingColumns>
where
    T: GenericClient,
{
    let mut columns_from_db: HashSet<SimpleColumn> = HashSet::new();
    for row in connection
        .query(
            "
            SELECT
                concat('public.', table_name) as table_name,
                column_name as column_name
            FROM information_schema.columns
            WHERE table_schema = 'public'
            ORDER BY table_name, column_name;",
            &[],
        )
        .unwrap()
    {
        let table_name: String = row.get("table_name");
        let column_name: String = row.get("column_name");
        columns_from_db.insert(SimpleColumn {
            table_name: table_name,
            column_name: column_name,
        });
    }

    let columns_from_strategy_file: HashSet<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table, columns)| {
            return columns.iter().map(|(column, _)| SimpleColumn {
                table_name: table.to_string(),
                column_name: column.to_string(),
            });
        })
        .collect();

    let in_strategy_file_but_not_db: Vec<_> = columns_from_strategy_file
        .difference(&columns_from_db)
        .map(|a| a.clone())
        .collect();

    let in_db_but_not_strategy_file: Vec<_> = columns_from_db
        .difference(&columns_from_strategy_file)
        .map(|a| a.clone())
        .collect();
    match (
        in_db_but_not_strategy_file.len(),
        in_strategy_file_but_not_db.len(),
    ) {
        (0, 0) => Ok(()),
        (0, _) => Err(MissingColumns {
            missing_from_db: Some(in_strategy_file_but_not_db),
            missing_from_strategy_file: None,
        }),
        (_, 0) => Err(MissingColumns {
            missing_from_db: None,
            missing_from_strategy_file: Some(in_db_but_not_strategy_file),
        }),
        (_, _) => Err(MissingColumns {
            missing_from_db: Some(in_strategy_file_but_not_db),
            missing_from_strategy_file: Some(in_db_but_not_strategy_file),
        }),
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

fn read_file(file_name: String) -> serde_json::Result<Vec<StrategyInFile>> {
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
    use crate::parsers::transformer::TransformerType;
    use postgres::Transaction;

    #[test]
    fn meh() {
        run_test(|connection| {
            let strategies = HashMap::from([
                (
                    "public.person".to_string(),
                    HashMap::from([
                        ("id".to_string(), create_transformer()),
                        ("first_name".to_string(), create_transformer()),
                    ]),
                ),
                (
                    "public.location".to_string(),
                    HashMap::from([
                        ("id".to_string(), create_transformer()),
                        ("post_code".to_string(), create_transformer()),
                    ]),
                ),
            ]);

            let result = generate(strategies, connection);
            println!("{:?}", result);
            assert!(false);
        });
    }

    fn create_transformer() -> Transformer {
        Transformer {
            name: TransformerType::Identity,
            args: None,
        }
    }

    fn run_test<T>(test: T) -> ()
    where
        T: Fn(&mut Transaction) -> (),
    {
        let mut conn = Client::connect(
            "postgresql://postgres:postgres@localhost:5432/postgres",
            NoTls,
        )
        .expect("expected connection to succeed");

        conn.batch_execute("DROP DATABASE anonymiser_test").unwrap();
        conn.batch_execute("CREATE DATABASE anonymiser_test")
            .unwrap();

        let mut anonymiser_test_conn = Client::connect(
            "postgresql://postgres:postgres@localhost:5432/anonymiser_test",
            NoTls,
        )
        .unwrap();

        let mut transaction = anonymiser_test_conn.transaction().unwrap();
        transaction
            .batch_execute(
                "
            CREATE TABLE person (
                id          SERIAL PRIMARY KEY,
                first_name  TEXT NOT NULL,
                last_name   TEXT NOT NULL
            )
        ",
            )
            .unwrap();

        transaction
            .batch_execute(
                "
            CREATE TABLE pet (
                id          SERIAL PRIMARY KEY,
                type  TEXT NOT NULL
            )
        ",
            )
            .unwrap();

        let result = test(&mut transaction);

        transaction.rollback().unwrap();
        return result;
    }
}
