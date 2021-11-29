use crate::parsers::copy_row::TableTransform;
use std::collections::HashMap;

pub struct RowParsingState {
    in_copy: bool,
    table_transforms: Option<TableTransform>,
}

pub fn initial_state() -> RowParsingState {
    RowParsingState {
        in_copy: false,
        table_transforms: None,
    }
}

pub fn parse<'a>(
    line: String,
    state: &'a mut RowParsingState,
    strategies: &'a HashMap<String, HashMap<String, String>>,
) -> &'a mut RowParsingState {
    if line.starts_with("COPY ") {
        let table_transforms = crate::parsers::copy_row::parse(line, strategies);
        print!("{:?}", table_transforms.table_name);
        state.in_copy = true;
        state.table_transforms = Some(table_transforms);
        return state;
    } else if line.starts_with("\\.") {
        state.in_copy = false;
        state.table_transforms = None;
        return state;
    } else {
        return state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_row_sets_status_to_being_in_copy_and_adds_transforms() {
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

        let mut initial_state = initial_state();
        let actual_status = parse(copy_row.to_string(), &mut initial_state, &strategies);
        assert!(actual_status.in_copy == true);
    }

    #[test]
    fn end_copy_row_sets_status_to_being_in_copy_and_adds_transforms() {
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

        let mut initial_state = initial_state();
        let actual_status = parse(copy_row.to_string(), &mut initial_state, &strategies);
        assert!(actual_status.in_copy == true);

        match &actual_status.table_transforms {
            None => assert!(false, "No table transforms set"),
            Some(actual_transforms) => assert_eq!(transforms, actual_transforms.transforms),
        };
    }
}
