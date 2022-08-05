use crate::parsers::sanitiser;
use crate::parsers::strategies::Strategies;
use crate::parsers::strategy_structs::ColumnInfo;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentTableTransforms {
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
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
                get_current_table_information(table, unsplit_columns, strategies)
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
    let column_list: Vec<String> = unsplit_columns
        .split(", ")
        .map(sanitiser::dequote_column_or_table_name_data)
        .collect();
    let columns = columns_from_strategy(strategies, &table_name, &column_list);

    CurrentTableTransforms {
        table_name,
        columns,
    }
}

fn columns_from_strategy(
    strategies: &Strategies,
    table_name: &str,
    column_list: &[String],
) -> Vec<ColumnInfo> {
    match strategies.for_table(table_name) {
        Some(columns) => column_list
            .iter()
            .map(|c| match columns.get(c) {
                Some(column_info) => column_info.clone(),
                None => panic!(
                    "No transform found for column: {:?} in table: {:?}",
                    c, table_name
                ),
            })
            .collect(),
        _ => panic!("No transforms found for table: {:?}", table_name),
    }
}

fn capture_to_item<'a, 'b>(capture: &'a regex::Captures, name: &'b str) -> Option<&'a str> {
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
        let column_infos = HashMap::from([
            ("id".to_string(), ColumnInfo::builder().build()),
            (
                "first_name".to_string(),
                ColumnInfo::builder()
                    .with_transformer(TransformerType::FakeFirstName, None)
                    .build(),
            ),
            (
                "last_name".to_string(),
                ColumnInfo::builder()
                    .with_transformer(TransformerType::FakeLastName, None)
                    .build(),
            ),
        ]);
        let strategies = Strategies::new_from("public.users".to_string(), column_infos);
        let parsed_copy_row = parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );

        let expected = CurrentTableTransforms {
            table_name: "public.users".to_string(),
            columns: vec![
                ColumnInfo::builder().build(),
                ColumnInfo::builder()
                    .with_transformer(TransformerType::FakeFirstName, None)
                    .build(),
                ColumnInfo::builder()
                    .with_transformer(TransformerType::FakeLastName, None)
                    .build(),
            ],
        };

        assert_eq!(expected.table_name, parsed_copy_row.table_name);
        assert_eq!(expected.columns, parsed_copy_row.columns);
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

        assert_eq!("public.references", parsed_copy_row.table_name);
        assert_eq!(vec![expected_column], parsed_copy_row.columns);
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
