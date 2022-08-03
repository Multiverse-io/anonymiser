use crate::parsers::copy_row;
use crate::parsers::copy_row::CurrentTableTransforms;
use crate::parsers::create_row;
use crate::parsers::state::*;
use crate::parsers::strategies::Strategies;
use crate::parsers::transformer;
use crate::parsers::types;
use crate::parsers::types::Column;
use itertools::join;

#[derive(Debug, PartialEq)]
enum RowType {
    Normal,
    CopyBlockStart,
    CopyBlockRow,
    CopyBlockEnd,
    CreateTableStart,
    CreateTableRow,
    CreateTableEnd,
}

fn row_type(line: &str, state: &Position) -> RowType {
    if line.starts_with("CREATE TABLE ") {
        RowType::CreateTableStart
    } else if line.starts_with("COPY ") {
        RowType::CopyBlockStart
    } else if line.starts_with("\\.") {
        RowType::CopyBlockEnd
    } else if line.starts_with(");") && matches!(state, Position::InCreateTable { .. }) {
        RowType::CreateTableEnd
    } else if matches!(state, Position::InCopy { .. }) {
        RowType::CopyBlockRow
    } else if matches!(state, Position::InCreateTable { .. }) {
        RowType::CreateTableRow
    } else {
        RowType::Normal
    }
}

pub fn parse(line: &str, state: &mut State, strategies: &Strategies) -> String {
    match (row_type(line, &state.position), state.position.clone()) {
        (RowType::CreateTableStart, _position) => {
            let table_name = create_row::parse(line);
            state.update_position(Position::InCreateTable {
                table_name,
                types: Vec::new(),
            });
            line.to_string()
        }
        (
            RowType::CreateTableRow,
            Position::InCreateTable {
                table_name,
                types: current_types,
            },
        ) => {
            state.update_position(Position::InCreateTable {
                table_name,
                types: add_create_table_row_to_types(line, current_types.to_vec()),
            });
            line.to_string()
        }
        (RowType::CreateTableEnd, _position) => {
            state.update_position(Position::Normal);
            line.to_string()
        }
        (RowType::CopyBlockStart, _position) => {
            let current_table = copy_row::parse(line, strategies);
            state.update_position(Position::InCopy { current_table });
            line.to_string()
        }
        (RowType::CopyBlockEnd, _position) => {
            state.update_position(Position::Normal);
            line.to_string()
        }
        (RowType::CopyBlockRow, Position::InCopy { current_table }) => {
            state.update_position(Position::InCopy {
                current_table: current_table.clone(),
            });
            transform_row(line, &current_table, &state.types)
        }

        (RowType::Normal, Position::Normal) => {
            state.update_position(Position::Normal);
            line.to_string()
        }
        (row_type, position) => {
            panic!(
                "omg! invalid combo of rowtype: {:?} and position: {:?}",
                row_type, position
            );
        }
    }
}

fn transform_row(line: &str, current_table: &CurrentTableTransforms, types: &Types) -> String {
    let column_values = split_row(line);

    let transformed = column_values.enumerate().map(|(i, value)| {
        //TODO sort this out
        let current_column = &current_table.columns[i];
        let _column_type = types.lookup(&current_table.table_name, "".to_string());

        transformer::transform(
            value,
            &current_column.transformer,
            &current_table.table_name,
        )
    });

    let mut joined = join(transformed, "\t");
    joined.push('\n');
    joined
}

fn add_create_table_row_to_types(line: &str, mut current_types: Vec<Column>) -> Vec<Column> {
    match types::parse(line) {
        None => (),
        Some(new_type) => current_types.push(new_type),
    }

    current_types
}

fn split_row(line: &str) -> std::str::Split<char> {
    return line.strip_suffix('\n').unwrap_or(line).split('\t');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{
        ColumnInfo, DataCategory, Transformer, TransformerType,
    };
    use crate::parsers::types::Type;
    use crate::test_builders::*;
    use std::collections::HashMap;

    #[test]
    fn create_table_start_row_is_parsed() {
        let create_table_row = "CREATE TABLE public.candidate_details (";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::from([]));

        let mut state = State::new();
        let transformed_row = parse(create_table_row, &mut state, &strategies);
        assert_eq!(
            state.position,
            Position::InCreateTable {
                table_name: "public.candidate_details".to_string(),
                types: Vec::new()
            }
        );
        assert_eq!(create_table_row, transformed_row);
    }

    #[test]
    fn create_table_mid_row_is_added_to_state() {
        let create_table_row = "password character varying(255)";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::from([]));

        let mut state = State {
            position: Position::InCreateTable {
                table_name: "public.users".to_string(),
                types: vec![Column {
                    name: "id".to_string(),
                    data_type: Type::integer(),
                }],
            },
            types: Types::new(HashMap::new()),
        };
        let transformed_row = parse(create_table_row, &mut state, &strategies);

        assert_eq!(
            state.position,
            Position::InCreateTable {
                table_name: "public.users".to_string(),
                types: vec![
                    Column {
                        name: "id".to_string(),
                        data_type: Type::integer()
                    },
                    Column {
                        name: "password".to_string(),
                        data_type: Type::character()
                    }
                ]
            }
        );
        assert_eq!(create_table_row, transformed_row);
    }

    #[test]
    fn non_type_create_table_row_is_ignored() {
        let create_table_row = "PARTITION BY something else";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::from([]));

        let mut state = State {
            position: Position::InCreateTable {
                table_name: "public.users".to_string(),
                types: vec![],
            },
            types: Types::new(HashMap::new()),
        };
        let transformed_row = parse(create_table_row, &mut state, &strategies);

        assert_eq!(
            state.position,
            Position::InCreateTable {
                table_name: "public.users".to_string(),
                types: vec![],
            }
        );
        assert_eq!(create_table_row, transformed_row);
    }

    #[test]
    fn end_of_a_create_table_row_changes_state() {
        let create_table_row = ");";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::from([]));

        let mut state = State {
            position: Position::InCreateTable {
                table_name: "public.users".to_string(),
                types: vec![Column {
                    name: "id".to_string(),
                    data_type: Type::integer(),
                }],
            },
            types: Types::new(HashMap::new()),
        };
        let transformed_row = parse(create_table_row, &mut state, &strategies);

        assert_eq!(state.position, Position::Normal);

        let expected_types = Types::new(HashMap::from([(
            "public.users".to_string(),
            HashMap::from([("id".to_string(), Type::integer())]),
        )]));
        assert_eq!(state.types, expected_types);
        assert_eq!(create_table_row, transformed_row);
    }

    #[test]
    fn copy_row_sets_status_to_being_in_copy_and_adds_transforms_in_the_correct_order_for_the_columns(
    ) {
        let id_column = ColumnInfo::builder().with_name("id").build();

        let first_name_column = ColumnInfo::builder()
            .with_name("first_name")
            .with_transformer(TransformerType::FakeFirstName, None)
            .build();
        let last_name_column = ColumnInfo::builder()
            .with_name("last_name")
            .with_transformer(TransformerType::FakeLastName, None)
            .build();

        let copy_row = "COPY public.users (id, first_name, last_name) FROM stdin;\n";

        let column_infos = HashMap::from([
            ("last_name".to_string(), last_name_column.clone()),
            ("id".to_string(), id_column.clone()),
            ("first_name".to_string(), first_name_column.clone()),
        ]);

        let strategies = Strategies::new_from("public.users".to_string(), column_infos.clone());

        let mut state = State::new();
        let transformed_row = parse(copy_row, &mut state, &strategies);

        assert_eq!(copy_row, transformed_row);

        match state.position {
            Position::InCopy { current_table } => {
                let expected_columns = vec![id_column, first_name_column, last_name_column];
                assert_eq!(expected_columns, current_table.columns)
            }
            _other => unreachable!("Position is not InCopy!"),
        };
    }

    #[test]
    fn end_copy_row_sets_status_to_being_in_copy_and_adds_transforms() {
        let end_copy_row = "\\.";
        let transforms = HashMap::from([
            (
                "id".to_string(),
                ColumnInfo {
                    data_category: DataCategory::General,
                    name: "id".to_string(),
                    transformer: Transformer {
                        name: TransformerType::Identity,
                        args: None,
                    },
                },
            ),
            (
                "first_name".to_string(),
                ColumnInfo {
                    data_category: DataCategory::General,
                    name: "first_name".to_string(),
                    transformer: Transformer {
                        name: TransformerType::FakeFirstName,
                        args: None,
                    },
                },
            ),
            (
                "last_name".to_string(),
                ColumnInfo {
                    data_category: DataCategory::General,
                    name: "last_name".to_string(),
                    transformer: Transformer {
                        name: TransformerType::FakeLastName,
                        args: None,
                    },
                },
            ),
        ]);
        let strategies = Strategies::new_from("public.users".to_string(), transforms);

        let mut state = State::new();
        let transformed_row = parse(end_copy_row, &mut state, &strategies);
        assert!(state.position == Position::Normal);
        assert_eq!(end_copy_row, transformed_row);
    }

    #[test]
    fn non_table_data_passes_through() {
        let non_table_data_row = "--this is a SQL comment";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::new());

        let mut state = State::new();
        let transformed_row = parse(non_table_data_row, &mut state, &strategies);
        assert!(state.position == Position::Normal);
        assert_eq!(non_table_data_row, transformed_row);
    }

    #[test]
    fn table_data_is_transformed() {
        let table_data_row = "123\tPeter\tPuckleberry\n";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::new());

        let mut state = State {
            position: Position::InCopy {
                current_table: CurrentTableTransforms {
                    table_name: "public.users".to_string(),
                    columns: vec![
                        ColumnInfo::builder()
                            .with_name("column_1")
                            .with_transformer(
                                TransformerType::Fixed,
                                Some(HashMap::from([("value".to_string(), "first".to_string())])),
                            )
                            .build(),
                        ColumnInfo::builder()
                            .with_name("column_2")
                            .with_transformer(
                                TransformerType::Fixed,
                                Some(HashMap::from([("value".to_string(), "second".to_string())])),
                            )
                            .build(),
                        ColumnInfo::builder()
                            .with_name("column_3")
                            .with_transformer(
                                TransformerType::Fixed,
                                Some(HashMap::from([("value".to_string(), "third".to_string())])),
                            )
                            .build(),
                    ],
                },
            },
            types: Types::new(HashMap::new()),
        };
        let transformed_row = parse(table_data_row, &mut state, &strategies);
        assert_eq!("first\tsecond\tthird\n", transformed_row);
    }

    #[test]
    fn transforms_array_fields() {
        let table_data_row = "{\"My string\"}\n";
        let strategies = Strategies::new_from("public.users".to_string(), HashMap::new());

        let mut state = State {
            position: Position::InCopy {
                current_table: CurrentTableTransforms {
                    table_name: "public.users".to_string(),
                    columns: vec![ColumnInfo::builder()
                        .with_transformer(TransformerType::Scramble, None)
                        .build()],
                },
            },
            types: Types::new(HashMap::new()),
        };
        let processed_row = parse(table_data_row, &mut state, &strategies);
        println!("{}", processed_row);
        assert!(table_data_row != processed_row);
    }
}
