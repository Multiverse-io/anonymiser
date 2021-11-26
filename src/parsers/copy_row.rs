use lazy_static::lazy_static;
use regex::Regex;

pub struct TableTransform {
    table_name: String,
    transforms: Vec<String>,
}

pub fn parse(copy_row: String) -> TableTransform {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"COPY (?P<table>.*) \((?P<columns>.*)\)").unwrap();
    }
    match RE.captures(&copy_row) {
        Some(cap) => {
            let some_columns = capture_to_item(&cap, "columns");
            let some_table = capture_to_item(&cap, "table");
            match (some_table, some_columns) {
                (Some(table), Some(columns)) => {
                    let column_list: Vec<String> =
                        columns.split(", ").map(|s| s.to_string()).collect();

                    return TableTransform {
                        table_name: table.to_string(),
                        transforms: column_list,
                    };
                }
                (_, _) => panic!("Invalid Copy row format: {:?}", copy_row),
            };
        }
        _ => panic!("Invalid Copy row format: {:?}", copy_row),
    };
}

fn capture_to_item<'a, 'b>(capture: &'a regex::Captures, name: &'b str) -> Option<&'a str> {
    capture.name(name).map(|x| x.as_str())
}
