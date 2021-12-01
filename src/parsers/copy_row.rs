use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug)]
pub struct CurrentTable {
    pub table_name: String,
    pub transforms: Option<Vec<String>>,
}

pub fn parse(
    copy_row: &str,
    strategies: &HashMap<String, HashMap<String, String>>,
) -> CurrentTable {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"COPY (?P<table>.*) \((?P<columns>.*)\)").unwrap();
    }

    match RE.captures(&copy_row) {
        Some(cap) => {
            let some_columns = capture_to_item(&cap, "columns");
            let some_table = capture_to_item(&cap, "table");
            match (some_table, some_columns) {
                (Some(table), Some(unsplit_columns)) => {
                    return get_current_table_information(table, unsplit_columns, strategies)
                }

                (_, _) => panic!("Invalid Copy row format: {:?}", copy_row),
            };
        }
        _ => panic!("Invalid Copy row format: {:?}", copy_row),
    };
}

fn get_current_table_information(
    table: &str,
    unsplit_columns: &str,
    strategies: &HashMap<String, HashMap<String, String>>,
) -> CurrentTable {
    let table_name = table.replace("\"", "");
    let column_list: Vec<String> = unsplit_columns
        .split(", ")
        .map(|s| s.replace("\"", "").to_string())
        .collect();
    let transforms = transforms_from_strategy(strategies, &table_name, &column_list);

    if column_list.len() != transforms.len() {
        let columns_set: HashSet<_> = column_list.iter().collect();
        let transforms_set: HashSet<_> = transforms.iter().collect();
        let diff: Vec<_> = columns_set.difference(&transforms_set).collect();

        panic!(
            "Table: {:?} is missing transforms for columns!\n\tcolumns: {:?}",
            table_name, diff
        )
    }

    return CurrentTable {
        table_name: table_name,
        transforms: Some(transforms),
    };
}

fn transforms_from_strategy(
    strategies: &HashMap<String, HashMap<String, String>>,
    table_name: &str,
    column_list: &Vec<String>,
) -> Vec<String> {
    match strategies.get(table_name) {
        Some(transforms) => {
            return column_list
                .iter()
                .map(|c| match transforms.get(c) {
                    Some(column_transform) => return column_transform.to_string(),
                    None => panic!(
                        "No tranform found for column: {:?} in table: {:?}",
                        c, table_name
                    ),
                })
                .collect::<Vec<_>>();
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

    #[test]
    fn returns_transforms_for_table() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            (
                "first_name".to_string(),
                "first_name_transformer".to_string(),
            ),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), expected_transforms.clone())]);
        let parsed_copy_row = parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );

        let expected = CurrentTable {
            table_name: "public.users".to_string(),
            transforms: Some(vec![
                "None".to_string(),
                "first_name_transformer".to_string(),
                "last_name_transformer".to_string(),
            ]),
        };
        assert_eq!(expected.table_name, parsed_copy_row.table_name);
        assert_eq!(expected.transforms, parsed_copy_row.transforms);
    }

    #[test]
    fn removes_quotes_around_table_names() {
        let strategies = HashMap::from([(
            "public.references".to_string(),
            HashMap::from([("id".to_string(), "None".to_string())]),
        )]);

        let parsed_copy_row = parse("COPY public.\"references\" (id) FROM stdin;\n", &strategies);

        assert_eq!("public.references", parsed_copy_row.table_name);
    }

    #[test]
    fn removes_quotes_around_column_names() {
        let strategies = HashMap::from([(
            "public.users".to_string(),
            HashMap::from([("from".to_string(), "None".to_string())]),
        )]);

        let _parsed_copy_row = parse("COPY public.users (\"from\") FROM stdin;\n", &strategies);

        assert!(true, "we didn't panic!");
    }
    #[test]
    #[should_panic(expected = "Invalid Copy row format")]
    fn panics_if_copy_row_is_not_formatted_correctly() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), expected_transforms.clone())]);
        parse("COPY public.users INTO THE SEA", &strategies);
    }

    #[test]
    #[should_panic(
        expected = "No tranform found for column: \"first_name\" in table: \"public.users\""
    )]
    fn panics_if_there_arent_transforms_for_all_columns() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), expected_transforms.clone())]);
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );
    }
    #[test]
    #[should_panic(expected = "No transforms found for table: \"public.users\"")]
    fn panics_if_there_are_no_transforms_for_the_table() {
        let strategies = HashMap::from([(
            "public.something_unrelated".to_string(),
            HashMap::from([("id".to_string(), "None".to_string())]),
        )]);
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n",
            &strategies,
        );
    }
}
