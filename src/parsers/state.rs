use crate::parsers::copy_row::CurrentTableTransforms;
use crate::parsers::types::Column;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Types(pub HashMap<String, HashMap<String, String>>);

impl Types {
    pub fn new(initial: HashMap<String, HashMap<String, String>>) {
        Types(initial);
    }

    pub fn insert(&self, table_name: &String, thing: HashMap<String, String>) {}
    pub fn lookup(&self, table_name: &String, column_name: String) -> String {
        "om".to_string()
    }
}

pub struct State {
    pub position: Position,
    pub types: Types,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Position {
    Normal,
    InCopy {
        current_table: CurrentTableTransforms,
    },
    InCreateTable {
        table_name: String,
        types: Vec<Column>,
    },
}

impl State {
    pub fn new() -> State {
        State {
            position: Position::Normal,
            types: Types(HashMap::new()),
        }
    }

    pub fn update_position(&mut self, new_position: Position) {
        match (self.position.clone(), new_position.clone()) {
            (
                Position::InCreateTable {
                    table_name,
                    types: table_types,
                },
                Position::Normal,
            ) => {
                self.types.insert(
                    &table_name,
                    table_types
                        .iter()
                        .map(|c| (c.name.clone(), c.data_type.clone()))
                        .collect::<HashMap<String, String>>(),
                );
                println!("TYPES: {:?}", table_types);
            }

            (_, _) => (),
        };
        self.position = new_position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::types::Column;
    use std::collections::HashMap;

    #[test]
    fn new_creates_default_state() {
        let state = State::new();
        assert_eq!(state.position, Position::Normal);
        assert_eq!(state.types, HashMap::new());
    }

    #[test]
    fn update_position_modifies_position() {
        let mut state = State::new();
        let new_position = Position::InCopy {
            current_table: CurrentTableTransforms {
                table_name: "table-mc-tableface".to_string(),
                transforms: None,
            },
        };

        state.update_position(new_position.clone());
        assert_eq!(state.position, new_position);
    }

    #[test]
    fn if_updating_from_InCreateTable_to_Normal_updates_types() {
        let mut state = State {
            position: Position::InCreateTable {
                table_name: "table-mc-tableface".to_string(),
                types: vec![
                    Column {
                        name: "column".to_string(),
                        data_type: "bigint".to_string(),
                    },
                    Column {
                        name: "column_2".to_string(),
                        data_type: "timestamp with time zone".to_string(),
                    },
                ],
            },
            types: Types(HashMap::new()),
        };

        state.update_position(Position::Normal);

        assert_eq!(state.position, Position::Normal);
        assert_eq!(
            state.types,
            HashMap::from([(
                "table-mc-tableface".to_string(),
                HashMap::from([
                    ("column".to_string(), "bigint".to_string()),
                    (
                        "column_2".to_string(),
                        "timestamp with time zone".to_string()
                    )
                ])
            )])
        );
    }
}
