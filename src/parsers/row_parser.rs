use crate::parsers::copy_row::CurrentTable;
use itertools::join;
use std::collections::HashMap;

pub struct RowParsingState {
    in_copy: bool,
    current_table: Option<CurrentTable>,
}

pub fn initial_state() -> RowParsingState {
    RowParsingState {
        in_copy: false,
        current_table: None,
    }
}

pub fn parse<'line, 'state>(
    line: &'line str,
    state: &'state mut RowParsingState,
    strategies: &'state HashMap<String, HashMap<String, String>>,
) -> &'line str {
    if line.starts_with("COPY ") {
        let current_table = crate::parsers::copy_row::parse(&line, strategies);
        state.in_copy = true;
        state.current_table = Some(current_table);
        return line;
    } else if line.starts_with("\\.") {
        state.in_copy = false;
        state.current_table = None;
        return line;
    } else if state.in_copy {
        return transform_row(line, &state.current_table);
    } else {
        return line;
    }
}

fn transform_column_value<'line, 'state>(value: &'line str, transform: &'state str) -> &'line str {
    match transform {
        "None" => value,
        "TestData" => "TestData",
        _ => panic!("unhandled transform: {:?}", transform),
    }
}

fn transform_row<'line, 'state>(
    line: &'line str,
    maybe_current_table: &'state Option<CurrentTable>,
) -> &'line str {
    let current_table = maybe_current_table
        .as_ref()
        .expect("Something bad happened, we're inside a copy block but we haven't realised!");

    match &current_table.transforms {
        Some(transforms) => {
            let column_values = split_row(line);
            let transformed = column_values
                .enumerate()
                .map(|(i, value)| return transform_column_value(value, &transforms[i]));

            let joined = join(transformed, "\t");
            return &joined;
        }
        None => line,
    }
}

fn split_row<'line>(line: &'line str) -> std::str::Split<&str> {
    return line.split("\t");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_row_sets_status_to_being_in_copy_and_adds_transforms_in_the_correct_order_for_the_columns(
    ) {
        let copy_row = "COPY public.users (id, first_name, last_name) FROM stdin;\n";
        let transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            (
                "first_name".to_string(),
                "first_name_transformer".to_string(),
            ),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), transforms.clone())]);

        let mut state = initial_state();
        let processed_row = parse(copy_row, &mut state, &strategies);
        assert!(state.in_copy == true);
        assert_eq!(copy_row, processed_row);

        match &state.current_table {
            Some(current_table) => assert_eq!(
                Some(vec!(
                    "None".to_string(),
                    "first_name_transformer".to_string(),
                    "last_name_transformer".to_string()
                )),
                current_table.transforms
            ),
            None => assert!(false, "No table transforms set"),
        };
    }

    #[test]
    fn end_copy_row_sets_status_to_being_in_copy_and_adds_transforms() {
        let end_copy_row = "\\.";
        let transforms = HashMap::from([
            ("id".to_string(), "None".to_string()),
            (
                "first_name".to_string(),
                "first_name_transformer".to_string(),
            ),
            ("last_name".to_string(), "last_name_transformer".to_string()),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), transforms.clone())]);

        let mut state = initial_state();
        let processed_row = parse(end_copy_row, &mut state, &strategies);
        assert!(state.in_copy == false);
        assert_eq!(end_copy_row, processed_row);
        assert!(state.current_table.is_none());
    }

    #[test]
    fn non_table_data_passes_through() {
        let non_table_data_row = "--this is a SQL comment";
        let strategies = HashMap::from([("public.users".to_string(), HashMap::from([]))]);

        let mut state = initial_state();
        let processed_row = parse(non_table_data_row, &mut state, &strategies);
        assert!(state.in_copy == false);
        assert!(state.current_table.is_none());
        assert_eq!(non_table_data_row, processed_row);
    }

    #[test]
    fn table_data_is_transformed() {
        //TODO Write this!
        let table_data_row = "123\tPeter\tPuckleberry";
        let strategies = HashMap::from([("public.users".to_string(), HashMap::from([]))]);

        let mut state = RowParsingState {
            in_copy: true,
            current_table: Some(CurrentTable {
                table_name: "public.users".to_string(),
                transforms: Some(vec![
                    "TestData".to_string(),
                    "TestData".to_string(),
                    "TestData".to_string(),
                ]),
            }),
        };
        let processed_row = parse(table_data_row, &mut state, &strategies);
        assert_eq!("TestData\tTestData\tTestData", processed_row);
    }
}
