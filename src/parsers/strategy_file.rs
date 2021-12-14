use crate::parsers::strategy_structs::*;
use crate::parsers::strategy_validator;
use postgres::GenericClient;
use postgres::{Client, NoTls};
use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

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

    return strategy_validator::validate(strategies, columns_from_db);
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
    use crate::parsers::strategy_structs::TransformerType;
    use postgres::Transaction;

    #[test]
    #[ignore]
    fn meh() {
        //TODO write a test here!
        //TODO get sql on CI!
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
