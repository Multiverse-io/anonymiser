use crate::parsers::copy_row;
use crate::parsers::copy_row::CurrentTable;
use crate::parsers::transformer;
use crate::parsers::transformer::Transformer;
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
    strategies: &'state HashMap<String, HashMap<String, Transformer>>,
) -> String {
    if line.starts_with("COPY ") {
        let current_table = copy_row::parse(&line, strategies);
        state.in_copy = true;
        state.current_table = Some(current_table);
        return line.to_string();
    } else if line.starts_with("\\.") {
        state.in_copy = false;
        state.current_table = None;
        return line.to_string();
    } else if state.in_copy {
        return transform_row(line, &state.current_table);
    } else {
        return line.to_string();
    }
}

fn transform_row<'line, 'state>(
    line: &'line str,
    maybe_current_table: &'state Option<CurrentTable>,
) -> String {
    let current_table = maybe_current_table
        .as_ref()
        .expect("Something bad happened, we're inside a copy block but we haven't realised!");

    match &current_table.transforms {
        Some(transforms) => {
            let column_values = split_row(line);
            let transformed = column_values.enumerate().map(|(i, value)| {
                return transformer::transform(value, &transforms[i], &current_table.table_name);
            });

            let mut joined = join(transformed, "\t");
            joined.push('\n');
            return joined;
        }
        None => {
            //TODO test carriage returns etc. here
            return line.to_string();
        }
    }
}

fn split_row<'line>(line: &'line str) -> std::str::Split<&str> {
    return line.strip_suffix("\n").unwrap_or(line).split("\t");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::transformer::TransformerType;

    #[test]
    fn copy_row_sets_status_to_being_in_copy_and_adds_transforms_in_the_correct_order_for_the_columns(
    ) {
        let copy_row = "COPY public.users (id, first_name, last_name) FROM stdin;\n";
        let transforms = HashMap::from([
            (
                "id".to_string(),
                Transformer {
                    name: TransformerType::Identity,
                    args: None,
                },
            ),
            (
                "first_name".to_string(),
                Transformer {
                    name: TransformerType::FakeFirstName,
                    args: None,
                },
            ),
            (
                "last_name".to_string(),
                Transformer {
                    name: TransformerType::FakeLastName,
                    args: None,
                },
            ),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), transforms)]);

        let mut state = initial_state();
        let processed_row = parse(copy_row, &mut state, &strategies);
        assert!(state.in_copy == true);
        assert_eq!(copy_row, processed_row);

        match &state.current_table {
            Some(current_table) => assert_eq!(
                Some(vec!(
                    Transformer {
                        name: TransformerType::Identity,
                        args: None,
                    },
                    Transformer {
                        name: TransformerType::FakeFirstName,
                        args: None,
                    },
                    Transformer {
                        name: TransformerType::FakeLastName,
                        args: None,
                    },
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
            (
                "id".to_string(),
                Transformer {
                    name: TransformerType::Identity,
                    args: None,
                },
            ),
            (
                "first_name".to_string(),
                Transformer {
                    name: TransformerType::FakeFirstName,
                    args: None,
                },
            ),
            (
                "last_name".to_string(),
                Transformer {
                    name: TransformerType::FakeLastName,
                    args: None,
                },
            ),
        ]);
        let strategies = HashMap::from([("public.users".to_string(), transforms)]);

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
        let table_data_row = "123\tPeter\tPuckleberry\n";
        let strategies = HashMap::from([("public.users".to_string(), HashMap::from([]))]);

        let mut state = RowParsingState {
            in_copy: true,
            current_table: Some(CurrentTable {
                table_name: "public.users".to_string(),
                transforms: Some(vec![
                    Transformer {
                        name: TransformerType::Fixed,
                        args: Some(HashMap::from([("value".to_string(), "first".to_string())])),
                    },
                    Transformer {
                        name: TransformerType::Fixed,
                        args: Some(HashMap::from([("value".to_string(), "second".to_string())])),
                    },
                    Transformer {
                        name: TransformerType::Fixed,
                        args: Some(HashMap::from([("value".to_string(), "third".to_string())])),
                    },
                ]),
            }),
        };
        let processed_row = parse(table_data_row, &mut state, &strategies);
        assert_eq!("first\tsecond\tthird\n", processed_row);
    }
}
