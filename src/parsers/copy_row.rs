use crate::parsers::sanitiser;
use crate::parsers::strategies::Strategies;
use crate::parsers::strategies::TableStrategy;
use crate::parsers::strategy_structs::ColumnInfo;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentTableTransforms {
    pub table_name: String,
    pub table_transformers: TableTransformers,
    pub salt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableTransformers {
    ColumnTransformer(Vec<ColumnInfo>),
    Truncator,
}

pub fn parse(copy_row: &str, strategies: &Strategies) -> CurrentTableTransforms {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"COPY (?P<table>.*) \((?P<columns>.*)\)").unwrap();
    }

    if let Some(cap) = RE.captures(copy_row) {
        let some_columns = capture_to_item(&cap, "columns");
        let some_table = capture_to_item(&cap, "table");
        match (some_table, some_columns) {
            (Some(table), Some(unsplit_columns)) => {
                let mut current_table =
                    get_current_table_information(table, unsplit_columns, strategies);
                current_table.salt = strategies.salt_for_table(table).map(String::from);
                current_table
            }
            (_, _) => panic!("Invalid Copy row format: {:?}", copy_row),
        }
    } else {
        panic!("Invalid Copy row format: {:?}", copy_row);
    }
}

fn get_current_table_information(
    table: &str,
    unsplit_columns: &str,
    strategies: &Strategies,
) -> CurrentTableTransforms {
    let table_name = sanitiser::dequote_column_or_table_name_data(table);
    let column_name_list: Vec<String> = unsplit_columns
        .split(", ")
        .map(sanitiser::dequote_column_or_table_name_data)
        .collect();
    let table_transformers = table_strategy(strategies, &table_name, &column_name_list);
    let salt = strategies.salt_for_table(&table_name).map(String::from);

    CurrentTableTransforms {
        table_name,
        table_transformers,
        salt,
    }
}

fn table_strategy(
    strategies: &Strategies,
    table_name: &str,
    column_name_list: &[String],
) -> TableTransformers {
    let strategies_for_table = strategies.for_table(table_name);

    match strategies_for_table {
        Some(TableStrategy::Columns(columns_with_names, _)) => {
            let column_infos = column_name_list
                .iter()
                .map(|column_name| match columns_with_names.get(column_name) {
                    Some(column_info) => column_info.clone(),
                    None => panic!(
                        "No transform found for column: {:?} in table: {:?}",
                        column_name, table_name
                    ),
                })
                .collect();
            TableTransformers::ColumnTransformer(column_infos)
        }

        Some(TableStrategy::Truncate) => TableTransformers::Truncator,
        None => panic!("No transforms found for table: {:?}", table_name),
    }
}

fn capture_to_item<'a>(capture: &'a regex::Captures, name: &str) -> Option<&'a str> {
    capture
        .name(name)
        .map(|parsed_copy_row| parsed_copy_row.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, TransformerType};
    use std::collections::HashMap;

    #[test]
    fn returns_transforms_for_table() {
        let columns = vec![
            ColumnInfo::builder().with_name("id").build(),
            ColumnInfo::builder()
                .with_transformer(TransformerType::FakeFirstName, None)
                .with_name("first_name")
                .build(),
            ColumnInfo::builder()
                .with_transformer(TransformerType::FakeLastName, None)
                .with_name("last_name")
                .build(),
        ];
        let column_infos_with_name: HashMap<String, ColumnInfo> = columns
            .iter()
            .map(|column| (column.name.clone(), column.clone()))
            .collect();
        let strategies = Strategies::new_from("public.users".to_string(), column_infos_with_name);
        let parsed_copy_row = parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );

        let expected = CurrentTableTransforms {
            table_name: "public.users".to_string(),
            table_transformers: TableTransformers::ColumnTransformer(columns),
            salt: None,
        };

        assert_eq!(expected.table_name, parsed_copy_row.table_name);
        assert_eq!(
            expected.table_transformers,
            parsed_copy_row.table_transformers
        );
        assert_eq!(expected.salt, parsed_copy_row.salt);
    }

    #[test]
    fn removes_quotes_around_table_and_column_names() {
        let expected_column = ColumnInfo::builder().with_name("from").build();
        let strategies = Strategies::new_from(
            "public.references".to_string(),
            HashMap::from([("from".to_string(), expected_column.clone())]),
        );

        let parsed_copy_row = parse(
            "COPY public.\"references\" (\"from\") FROM stdin;\n",
            &strategies,
        );

        let expected_table_transformers =
            TableTransformers::ColumnTransformer(vec![expected_column]);

        assert_eq!("public.references", parsed_copy_row.table_name);
        assert_eq!(
            expected_table_transformers,
            parsed_copy_row.table_transformers
        );
        assert_eq!(None, parsed_copy_row.salt);
    }

    #[test]
    #[should_panic(expected = "Invalid Copy row format")]
    fn panics_if_copy_row_is_not_formatted_correctly() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), ColumnInfo::builder().build()),
            ("last_name".to_string(), ColumnInfo::builder().build()),
        ]);
        let strategies = Strategies::new_from("public.users".to_string(), expected_transforms);
        parse("COPY public.users INTO THE SEA", &strategies);
    }

    #[test]
    #[should_panic(
        expected = "No transform found for column: \"first_name\" in table: \"public.users\""
    )]
    fn panics_if_there_arent_transforms_for_all_columns() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), ColumnInfo::builder().build()),
            ("last_name".to_string(), ColumnInfo::builder().build()),
        ]);
        let strategies = Strategies::new_from("public.users".to_string(), expected_transforms);
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );
    }
    #[test]
    #[should_panic(expected = "No transforms found for table: \"public.users\"")]
    fn panics_if_there_are_no_transforms_for_the_table() {
        let expected_transforms =
            HashMap::from([("id".to_string(), ColumnInfo::builder().build())]);
        let strategies = Strategies::new_from("public.unrelated".to_string(), expected_transforms);
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );
    }
}
