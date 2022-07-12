use crate::parsers::strategy_structs::*;
use postgres::GenericClient;
use std::collections::HashSet;

pub fn parse<T>(connection: &mut T) -> HashSet<SimpleColumn>
where
    T: GenericClient,
{
    let mut columns_from_db: HashSet<SimpleColumn> = HashSet::new();
    for row in connection
        .query(
            "
            SELECT
                concat(c.table_schema, '.', c.table_name) as table_name,
                column_name as column_name,
                c.table_schema as schema_name
            FROM information_schema.columns c
            INNER JOIN information_schema.tables t on c.table_name = t.table_name and c.table_schema = t.table_schema
            WHERE c.table_schema NOT IN ('information_schema', 'pg_catalog')
            AND table_type = 'BASE TABLE'
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

    return columns_from_db;
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres::Transaction;
    use postgres::{Client, NoTls};

    #[test]
    #[ignore]
    //TODO enable on CI!!!
    fn can_read_db_columns() {
        run_test(|connection| {
            let result = parse(connection);
            assert_eq!(
                result,
                HashSet::from([
                    SimpleColumn {
                        table_name: "archived.old_table".to_string(),
                        column_name: "old_column".to_string()
                    },
                    SimpleColumn {
                        table_name: "archived.old_table".to_string(),
                        column_name: "id".to_string()
                    },
                    SimpleColumn {
                        table_name: "public.person".to_string(),
                        column_name: "id".to_string()
                    },
                    SimpleColumn {
                        table_name: "public.person".to_string(),
                        column_name: "first_name".to_string()
                    },
                    SimpleColumn {
                        table_name: "public.person".to_string(),
                        column_name: "last_name".to_string()
                    },
                    SimpleColumn {
                        table_name: "public.location".to_string(),
                        column_name: "id".to_string()
                    },
                    SimpleColumn {
                        table_name: "public.location".to_string(),
                        column_name: "post_code".to_string()
                    },
                ])
            );
        });
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
            .batch_execute("CREATE SCHEMA IF NOT EXISTS archived")
            .unwrap();
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
            CREATE TABLE location (
                id          SERIAL PRIMARY KEY,
                post_code  TEXT NOT NULL
            )
        ",
            )
            .unwrap();

        transaction
            .batch_execute(
                "
            CREATE TABLE archived.old_table (
                id          SERIAL PRIMARY KEY,
                old_column  TEXT NOT NULL
            )
        ",
            )
            .unwrap();

        transaction
            .batch_execute(
                "
            CREATE VIEW archived.location AS SELECT id, post_code FROM public.location
        ",
            )
            .unwrap();

        let result = test(&mut transaction);

        transaction.rollback().unwrap();
        return result;
    }
}
