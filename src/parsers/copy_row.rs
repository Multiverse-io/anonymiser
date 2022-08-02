use crate::parsers::strategies::Strategies;
use crate::parsers::strategy_structs::Transformer;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentTableTransforms {
    pub table_name: String,
    pub transforms: Option<Vec<Transformer>>,
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
    let table_name = table.replace('\"', "");
    let column_list: Vec<String> = unsplit_columns
        .split(", ")
        .map(|s| s.replace('\"', ""))
        .collect();
    let transforms = transforms_from_strategy(strategies, &table_name, &column_list);

    CurrentTableTransforms {
        table_name,
        transforms: Some(transforms),
    }
}

fn transforms_from_strategy(
    strategies: &Strategies,
    table_name: &str,
    column_list: &[String],
) -> Vec<Transformer> {
    match strategies.for_table(table_name) {
        Some(columns) => {
            return column_list
                .iter()
                .map(|c| match columns.get(c) {
                    Some(column_info) => column_info.transformer.clone(),
                    None => panic!(
                        "No transform found for column: {:?} in table: {:?}",
                        c, table_name
                    ),
                })
                .collect();
        }
        _ => panic!("No transforms found for table: {:?}", table_name),
    };
}

fn capture_to_item<'a, 'b>(capture: &'a regex::Captures, name: &'b str) -> Option<&'a str> {
    capture
        .name(name)
        .map(|parsed_copy_row| parsed_copy_row.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, DataCategory, TransformerType};
    use std::collections::HashMap;

    #[test]
    fn returns_transforms_for_table() {
        let column_infos = HashMap::from([
            (
                "id".to_string(),
                create_column_info(TransformerType::Identity),
            ),
            (
                "first_name".to_string(),
                create_column_info(TransformerType::FakeFirstName),
            ),
            (
                "last_name".to_string(),
                create_column_info(TransformerType::FakeLastName),
            ),
        ]);
        let strategies = Strategies::new_from("public.users".to_string(), column_infos);
        let parsed_copy_row = parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );

        let expected = CurrentTableTransforms {
            table_name: "public.users".to_string(),
            transforms: Some(vec![
                create_transformer(TransformerType::Identity),
                create_transformer(TransformerType::FakeFirstName),
                create_transformer(TransformerType::FakeLastName),
            ]),
        };
        assert_eq!(expected.table_name, parsed_copy_row.table_name);
        assert_eq!(
            expected.transforms.unwrap(),
            parsed_copy_row.transforms.unwrap()
        );
    }

    #[test]
    fn removes_quotes_around_table_names() {
        let strategies = Strategies::new_from(
            "public.references".to_string(),
            HashMap::from([(
                "id".to_string(),
                create_column_info(TransformerType::Identity),
            )]),
        );

        let parsed_copy_row = parse("COPY public.\"references\" (id) FROM stdin;\n", &strategies);

        assert_eq!("public.references", parsed_copy_row.table_name);
    }

    #[test]
    fn doesnt_panic_with_quotes_around_column_names() {
        let strategies = Strategies::new_from(
            "public.users".to_string(),
            HashMap::from([
                (
                    "id".to_string(),
                    create_column_info(TransformerType::Identity),
                ),
                (
                    "from".to_string(),
                    create_column_info(TransformerType::Identity),
                ),
            ]),
        );

        let _parsed_copy_row = parse("COPY public.users (\"from\") FROM stdin;\n", &strategies);
    }

    fn create_column_info(name: TransformerType) -> ColumnInfo {
        ColumnInfo {
            name: "column1".to_string(),
            transformer: create_transformer(name),
            data_category: DataCategory::General,
        }
    }
    fn create_transformer(name: TransformerType) -> Transformer {
        Transformer { name, args: None }
    }
    #[test]
    #[should_panic(expected = "Invalid Copy row format")]
    fn panics_if_copy_row_is_not_formatted_correctly() {
        let expected_transforms = HashMap::from([
            (
                "id".to_string(),
                create_column_info(TransformerType::Identity),
            ),
            (
                "last_name".to_string(),
                create_column_info(TransformerType::FakeLastName),
            ),
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
            (
                "id".to_string(),
                create_column_info(TransformerType::Identity),
            ),
            (
                "last_name".to_string(),
                create_column_info(TransformerType::FakeLastName),
            ),
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
        let strategies = Strategies::new_from(
            "public.something_unrelated".to_string(),
            HashMap::from([(
                "id".to_string(),
                create_column_info(TransformerType::Identity),
            )]),
        );
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );
    }
}
