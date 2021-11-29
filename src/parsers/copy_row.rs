use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TableTransform {
    pub table_name: String,
    pub transforms: Option<HashMap<String, String>>,
}

pub fn parse(
    copy_row: String,
    strategies: &HashMap<String, HashMap<String, String>>,
) -> TableTransform {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"COPY (?P<table>.*) \((?P<columns>.*)\)").unwrap();
    }

    match RE.captures(&copy_row) {
        Some(cap) => {
            let some_columns = capture_to_item(&cap, "columns");
            let some_table = capture_to_item(&cap, "table");
            match (some_table, some_columns) {
                (Some(table), Some(unsplit_columns)) => {
                    return create_table_transform(table, unsplit_columns, strategies)
                }

                (_, _) => panic!("Invalid Copy row format: {:?}", copy_row),
            };
        }
        _ => panic!("Invalid Copy row format: {:?}", copy_row),
    };
}

fn create_table_transform(
    table: &str,
    unsplit_columns: &str,
    strategies: &HashMap<String, HashMap<String, String>>,
) -> TableTransform {
    let table_name = table.replace("\"", "");
    let column_list: Vec<String> = unsplit_columns.split(", ").map(|s| s.to_string()).collect();
    let transforms = transforms_from_strategy(strategies, &table_name);

    if column_list.len() != transforms.len() {
        //TODO sort this!
        //       let sorted_list = column_list.clone().sort();
        //     print!("{:?}", sorted_list);
        panic!(
            "Table: {:?} is missing transforms for columns!\n\tcolumns: {:?}\n\ttransforms: {:?}",
            table_name,
            column_list,
            transforms.keys()
        )
    }

    return TableTransform {
        table_name: table_name,
        transforms: Some(transforms),
    };
}

fn transforms_from_strategy(
    strategies: &HashMap<String, HashMap<String, String>>,
    table_name: &str,
) -> HashMap<String, String> {
    match strategies.get(table_name) {
        Some(transforms) => return transforms.clone(), //TODO the clone seems bad here
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
            "COPY public.users (id, first_name, last_name) FROM stdin;\n".to_string(),
            &strategies,
        );

        let expected = TableTransform {
            table_name: "public.users".to_string(),
            transforms: expected_transforms,
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

        let parsed_copy_row = parse(
            "COPY public.\"references\" (id) FROM stdin;\n".to_string(),
            &strategies,
        );

        assert_eq!("public.references", parsed_copy_row.table_name);
    }
    #[test]
    #[should_panic(expected = "Invalid Copy row format")]
    fn panics_if_copy_row_is_not_formatted_correctly() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), expected_transforms.clone())]);
        parse("COPY public.users INTO THE SEA".to_string(), &strategies);
    }

    #[test]
    #[should_panic(expected = "Table: \"public.users\" is missing transforms for columns!")]
    fn panics_if_there_arent_transforms_for_all_columns() {
        let expected_transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), expected_transforms.clone())]);
        parse(
            "COPY public.users (id, first_name, last_name) FROM stdin;\n".to_string(),
            &strategies,
        );
    }
}
