#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: String,
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
    println!("{:?}", line);
    let trimmed_line = match line.strip_suffix(",") {
        None => line,
        Some(trimmed_line) => trimmed_line,
    };

    let mut bits = trimmed_line.split(" ");
    let name = bits
        .next()
        .expect("Not expecting an empty row inside a CREATE TABLE statement!");

    if !is_non_column_definition(name) {
        let rest: String = bits
            .take_while(|w| match w {
                &"COLLATE" => false,
                &"COMPRESSION" => false,
                &"NOT" => false,
                &"NULL" => false,
                &"CHECK" => false,
                &"DEFAULT" => false,
                &"GENERATED" => false,
                &"UNIQUE" => false,
                &"PRIMARY" => false,
                &"REFERENCES" => false,
                &"DEFERRABLE" => false,
                &"INITIALLY" => false,
                _ => true,
            })
            .collect::<Vec<&str>>()
            .join(" ");

        return Some(Column {
            name: name.to_string(),
            data_type: rest,
        });
    } else {
        return None;
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn create_table_start_row_is_parsed() {
        let row = "password character varying(255),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "password");
        assert_eq!(parsed.data_type, "character varying(255)");
    }

    //These are written based on the BNF here: https://www.postgresql.org/docs/current/sql-createtable.html

    #[test]
    fn with_column_COMPRESSION_modifier() {
        let row = "id bigint COMPRESSION pglz,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_COLLATE_modifier() {
        let row = "id bigint COLLATE \"es_ES\",";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_NOT_NULL_modifier() {
        let row = "id bigint NOT NULL,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_NULL_modifier() {
        let row = "id bigint NULL,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_CHECK_modifier() {
        let row = "id bigint CHECK (id >= 0),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_DEFAULT_modifier() {
        let row = "is_great_fun boolean DEFAULT false,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "is_great_fun");
        assert_eq!(parsed.data_type, "boolean");
    }

    #[test]
    fn with_column_constraint_GENERATED_modifier() {
        let row = "id bigint GENERATED ALWAYS AS IDENTIFY,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_UNIQUE_modifier() {
        let row = "id bigint UNIQUE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_PRIMARY_KEY_modifier() {
        let row = "id bigint PRIMARY KEY,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_REFERENCES_modifier() {
        let row = "id bigint REFERENCES products (product_no),";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_DEFERRABLE_modifier() {
        let row = "id bigint DEFERRABLE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_NOT_DEFERRABLE_modifier() {
        let row = "id bigint NOT DEFERRABLE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }
    #[test]
    fn with_column_constraint_INITIALLY_DEFERRED_modifier() {
        let row = "id bigint INITIALLY DEFERRED,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
    }

    #[test]
    fn with_column_constraint_INITIALLY_IMMEDIATE_modifier() {
        let row = "id bigint INITIALLY IMMEDIATE,";
        let parsed = parse(row).expect("Expected a column back! but got None");
        assert_eq!(parsed.name, "id");
        assert_eq!(parsed.data_type, "bigint");
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
}
