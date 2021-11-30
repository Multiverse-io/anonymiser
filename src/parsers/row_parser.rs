use crate::parsers::copy_row::CurrentTable;
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

pub fn parse<'a>(
    line: String,
    state: &'a mut RowParsingState,
    strategies: &'a HashMap<String, HashMap<String, String>>,
) -> String {
    if line.starts_with("COPY ") {
        let current_table = crate::parsers::copy_row::parse(&line, strategies);
        state.in_copy = true;
        state.current_table = Some(current_table);
        return line;
    } else if line.starts_with("\\.") {
        state.in_copy = false;
        state.current_table = None;
        return line;
    } else {
        return line;
    }
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
        let processed_row = parse(copy_row.to_string(), &mut state, &strategies);
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
        let processed_row = parse(end_copy_row.to_string(), &mut state, &strategies);
        assert!(state.in_copy == false);
        assert_eq!(end_copy_row, processed_row);
        match &state.current_table {
            None => assert!(true),
            Some(_) => assert!(false, "end row should unset the current table"),
        };
    }
}
