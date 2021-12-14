use crate::parsers::strategy_structs::*;
use std::collections::HashSet;
pub fn validate(
    strategies: &Strategies,
    columns_from_db: HashSet<SimpleColumn>,
) -> Result<(), MissingColumns> {
    let columns_from_strategy_file: HashSet<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table, columns)| {
            return columns.iter().map(|(column, _)| SimpleColumn {
                table_name: table.to_string(),
                column_name: column.to_string(),
            });
        })
        .collect();

    let in_strategy_file_but_not_db: Vec<_> = columns_from_strategy_file
        .difference(&columns_from_db)
        .map(|a| a.clone())
        .collect();

    let in_db_but_not_strategy_file: Vec<_> = columns_from_db
        .difference(&columns_from_strategy_file)
        .map(|a| a.clone())
        .collect();
    match (
        in_db_but_not_strategy_file.len(),
        in_strategy_file_but_not_db.len(),
    ) {
        (0, 0) => Ok(()),
        (0, _) => Err(MissingColumns {
            missing_from_db: Some(in_strategy_file_but_not_db),
            missing_from_strategy_file: None,
        }),
        (_, 0) => Err(MissingColumns {
            missing_from_db: None,
            missing_from_strategy_file: Some(in_db_but_not_strategy_file),
        }),
        (_, _) => Err(MissingColumns {
            missing_from_db: Some(in_strategy_file_but_not_db),
            missing_from_strategy_file: Some(in_db_but_not_strategy_file),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, Transformer, TransformerType};
    use std::collections::HashMap;

    #[test]
    fn returns_ok_with_matching_fields() {
        let strategies = HashMap::from([
            create_strategy("public.person", "first_name"),
            create_strategy("public.location", "postcode"),
        ]);

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.location", "postcode"),
        ]);

        let result = validate(&strategies, columns_from_db);

        assert!(result.is_ok());
    }

    #[test]
    fn returns_fields_missing_from_strategy_file_that_are_in_the_db() {
        let strategies = HashMap::from([create_strategy("public.person", "first_name")]);

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.location", "postcode"),
        ]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_db.is_none());
        assert_eq!(
            error.missing_from_strategy_file.unwrap(),
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    #[test]
    fn returns_fields_missing_from_the_db_but_are_in_the_strategy_file() {
        let strategies = HashMap::from([
            create_strategy("public.person", "first_name"),
            create_strategy("public.location", "postcode"),
        ]);

        let columns_from_db = HashSet::from([create_simple_column("public.person", "first_name")]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_strategy_file.is_none());
        assert_eq!(
            error.missing_from_db.unwrap(),
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    #[test]
    fn returns_fields_missing_both() {
        let strategies = HashMap::from([create_strategy("public.person", "first_name")]);

        let columns_from_db = HashSet::from([create_simple_column("public.location", "postcode")]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.missing_from_strategy_file.unwrap(),
            vec!(create_simple_column("public.location", "postcode"))
        );
        assert_eq!(
            error.missing_from_db.unwrap(),
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    fn create_strategy(
        table_name: &str,
        column_name: &str,
    ) -> (String, HashMap<String, ColumnInfo>) {
        return (
            table_name.to_string(),
            HashMap::from([(
                column_name.to_string(),
                ColumnInfo {
                    data_type: DataType::General,
                    transformer: Transformer {
                        name: TransformerType::Identity,
                        args: None,
                    },
                },
            )]),
        );
    }

    fn create_simple_column(table_name: &str, column_name: &str) -> SimpleColumn {
        return SimpleColumn {
            table_name: table_name.to_string(),
            column_name: column_name.to_string(),
        };
    }
}
