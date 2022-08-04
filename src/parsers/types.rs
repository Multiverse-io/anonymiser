#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    SingleValue { sub_type: SubType },
    Array { sub_type: SubType },
}

impl Type {
    pub fn array(sub_type: SubType) -> Self {
        Type::Array { sub_type }
    }

    pub fn single_value(sub_type: SubType) -> Self {
        Type::SingleValue { sub_type }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubType {
    //TODO JSON?!?
    Character,
    Integer,
    Unknown { underlying_type: String },
}

fn is_non_column_definition(first_word: &str) -> bool {
    let non_column_starting_words = [
        "NOT",
        "CONSTRAINT",
        "CHECK",
        "UNIQUE",
        "PRIMARY",
        "EXCLUDE",
        "FOREIGN",
        "DEFERRABLE",
        "INITIALLY",
        "INHERITS",
        "ON",
        "PARTITION",
        "TABLESPACE",
        "USING",
        "WITH",
    ];
    non_column_starting_words.contains(&first_word)
}

pub fn parse(line: &str) -> Option<Column> {
    let mut trimmed_line = line.trim();
    trimmed_line = match trimmed_line.strip_suffix(',') {
        None => trimmed_line,
        Some(stripped_line) => stripped_line,
    };

    let mut bits = trimmed_line.split(' ');
    let name = bits
        .next()
        .expect("Not expecting an empty row inside a CREATE TABLE statement!");

    if !is_non_column_definition(name) {
        let rest: String = bits
            .take_while(|w| {
                !matches!(
                    *w,
                    "COLLATE"
                        | "COMPRESSION"
                        | "NOT"
                        | "NULL"
                        | "CHECK"
                        | "DEFAULT"
                        | "GENERATED"
                        | "UNIQUE"
                        | "PRIMARY"
                        | "REFERENCES"
                        | "DEFERRABLE"
                        | "INITIALLY"
                )
            })
            .collect::<Vec<&str>>()
            .join(" ");

        Some(Column {
            name: name.to_string(),
            data_type: string_to_type(rest),
        })
    } else {
        None
    }
}

fn string_to_type(type_string: String) -> Type {
    let sub_type = if type_string.starts_with("character") {
        SubType::Character
    } else if type_string.starts_with("bigint") {
        SubType::Integer
    } else {
        SubType::Unknown {
            underlying_type: type_string.clone(),
        }
    };

    if type_string.ends_with("[]") {
        Type::array(sub_type)
    } else {
        Type::single_value(sub_type)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn trims_whitespace() {
        let row = "    password character varying(255),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "password");
        assert_eq!(parsed.data_type, Type::character());
    }

    #[test]
    fn parses_character_type() {
        let row = "password character varying(255),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "password");
        assert_eq!(parsed.data_type, Type::character());
    }

    #[test]
    fn parses_other_type_as_unknown() {
        let row = "check boolean,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "check");
        assert_eq!(parsed.data_type, Type::unknown("boolean".to_string()));
    }

    #[test]
    fn parses_date_type() {
        let row = "inserted_at timestamp with time zone NOT NULL,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "inserted_at");
        assert_eq!(
            parsed.data_type,
            Type::unknown("timestamp with time zone".to_string())
        );
    }

    #[test]
    fn parses_array_of_character_type() {
        let row = "password character varying(255)[],";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "password");
        assert_eq!(parsed.data_type, Type::array(SubType::Character));
    }

    //These are written based on the BNF here: https://www.postgresql.org/docs/current/sql-createtable.html

    #[test]
    fn with_column_COMPRESSION_modifier() {
        let row = "id bigint COMPRESSION pglz,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_COLLATE_modifier() {
        let row = "id bigint COLLATE \"es_ES\",";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_NOT_NULL_modifier() {
        let row = "id bigint NOT NULL,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_NULL_modifier() {
        let row = "id bigint NULL,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_CHECK_modifier() {
        let row = "id bigint CHECK (id >= 0),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_DEFAULT_modifier() {
        let row = "is_great_fun boolean DEFAULT false,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "is_great_fun");
        assert_eq!(parsed.data_type, Type::unknown("boolean".to_string()));
    }

    #[test]
    fn with_column_constraint_GENERATED_modifier() {
        let row = "id bigint GENERATED ALWAYS AS IDENTIFY,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_UNIQUE_modifier() {
        let row = "id bigint UNIQUE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_PRIMARY_KEY_modifier() {
        let row = "id bigint PRIMARY KEY,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_REFERENCES_modifier() {
        let row = "id bigint REFERENCES products (product_no),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_DEFERRABLE_modifier() {
        let row = "id bigint DEFERRABLE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_NOT_DEFERRABLE_modifier() {
        let row = "id bigint NOT DEFERRABLE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }
    #[test]
    fn with_column_constraint_INITIALLY_DEFERRED_modifier() {
        let row = "id bigint INITIALLY DEFERRED,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_column_constraint_INITIALLY_IMMEDIATE_modifier() {
        let row = "id bigint INITIALLY IMMEDIATE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, Type::integer());
    }

    #[test]
    fn with_table_constraint_CONSTRAINT_modifier() {
        let row = "CONSTRAINT id CHECK ((id > 0)),";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_CHECK_modifier() {
        let row = "CHECK not_sure_here, can't find an example!,";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_UNIQUE_modifier() {
        let row = "UNIQUE (name),";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_PRIMARY_KEY_modifier() {
        let row = "PRIMARY KEY (something or other)";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_EXCLUDE_modifier() {
        let row = "EXCLUDE USING gist (c WITH &&)";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_FOREIGN_KEY_modifier() {
        let row = "FOREIGN KEY again, not sure what goes here";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_DEFERRABLE_modifier() {
        let row = "DEFERRABLE";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_NOT_DEFERRABLE_modifier() {
        let row = "NOT DEFERRABLE";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_table_constraint_INITIALLY_modifier() {
        let row = "INITIALLY DEFERRED";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_INHERITS_option() {
        let row = "INHERITS parent_table some other stuff here";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_PARTITION_BY_option() {
        let row = "PARTITION BY RANGE some stuff here";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_USING_option() {
        let row = "USING method";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_WITH_option() {
        let row = "WITH not sure here";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_ON_COMMIT_option() {
        let row = "ON COMMIT DELETE ROWS";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    #[test]
    fn with_TABLESPACE_option() {
        let row = "TABLESPACE tablespace_name";
        let parsed = parse(row);
        assert!(parsed.is_none());
    }

    impl Type {
        pub fn integer() -> Self {
            Type::SingleValue {
                sub_type: SubType::Integer,
            }
        }

        pub fn character() -> Self {
            Type::SingleValue {
                sub_type: SubType::Character,
            }
        }

        pub fn unknown(underlying_type: String) -> Self {
            Type::SingleValue {
                sub_type: SubType::Unknown { underlying_type },
            }
        }
    }
}
