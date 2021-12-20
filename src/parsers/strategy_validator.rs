use crate::parsers::strategy_structs::*;
use std::collections::HashSet;
fn create_simple_column(column_name: &str, table_name: &str) -> SimpleColumn {
    return SimpleColumn {
        table_name: table_name.to_string(),
        column_name: column_name.to_string(),
    };
}
pub fn validate(
    strategies: &Strategies,
    columns_from_db: HashSet<SimpleColumn>,
) -> Result<(), MissingColumns> {
    //TODO probably dont iterate over and over again!

    let unanonymised_pii: Vec<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table_name, columns)| {
            return columns
                .iter()
                .filter(|(_, column_info)| {
                    return (column_info.data_type == DataType::PotentialPii
                        || column_info.data_type == DataType::Pii)
                        && column_info.transformer.name == TransformerType::Identity;
                })
                .map(|(column_name, _)| create_simple_column(column_name, table_name));
        })
        .collect();
    let unknown_data_types: Vec<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table_name, columns)| {
            return columns
                .iter()
                .filter(|(_, column_info)| column_info.data_type == DataType::Unknown)
                .map(|(column_name, _)| create_simple_column(column_name, table_name));
        })
        .collect();

    let error_transformer_types: Vec<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table_name, columns)| {
            return columns
                .iter()
                .filter(|(_, column_info)| column_info.transformer.name == TransformerType::Error)
                .map(|(column_name, _)| create_simple_column(column_name, table_name));
        })
        .collect();

    let columns_from_strategy_file: HashSet<SimpleColumn> = strategies
        .iter()
        .flat_map(|(table, columns)| {
            return columns
                .iter()
                .map(|(column, _)| create_simple_column(column, table));
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
        unknown_data_types.len(),
        error_transformer_types.len(),
        unanonymised_pii.len(),
    ) {
        (0, 0, 0, 0, 0) => Ok(()),
        _ => {
            return Err(MissingColumns {
                missing_from_db: add_if_present(in_strategy_file_but_not_db),
                missing_from_strategy_file: add_if_present(in_db_but_not_strategy_file),
                unknown_data_types: add_if_present(unknown_data_types),
                error_transformer_types: add_if_present(error_transformer_types),
                unanonymised_pii: add_if_present(unanonymised_pii),
            });
        }
    }
}

fn add_if_present(list: Vec<SimpleColumn>) -> Option<Vec<SimpleColumn>> {
    if list.len() > 0 {
        let mut new_list = list.clone();
        new_list.sort();
        return Some(new_list);
    } else {
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, Transformer, TransformerType};
    use std::collections::HashMap;

    #[test]
    fn returns_ok_with_matching_fields() {
        let mut strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        add_table(
            &mut strategies,
            "public.location",
            [create_column("postcode")].into_iter(),
        );

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.location", "postcode"),
        ]);

        let result = validate(&strategies, columns_from_db);

        assert!(result.is_ok());
    }

    #[test]
    fn returns_fields_missing_from_strategy_file_that_are_in_the_db() {
        let strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

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
        let mut strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        add_table(
            &mut strategies,
            "public.location",
            [create_column("postcode")].into_iter(),
        );

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
        let strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

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

    #[test]
    fn returns_columns_missing_data_type() {
        let strategies = create_strategy(
            "public.person",
            [create_strategy_with_data_type(
                "first_name",
                DataType::Unknown,
            )]
            .into_iter(),
        );

        let columns_from_db = HashSet::from([create_simple_column("public.person", "first_name")]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.unknown_data_types.unwrap(),
            vec!(create_simple_column("public.person", "first_name"))
        );
    }
    #[test]
    fn returns_columns_with_error_transformer_types() {
        let strategies = create_strategy(
            "public.person",
            [create_column_with_transformer_type(
                "first_name",
                TransformerType::Error,
            )]
            .into_iter(),
        );

        let columns_from_db = HashSet::from([create_simple_column("public.person", "first_name")]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.error_transformer_types.unwrap(),
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn returns_pii_columns_with_identity_transformer() {
        let strategies = create_strategy(
            "public.person",
            [
                create_column_with_data_and_transfromer_type(
                    "first_name",
                    DataType::Pii,
                    TransformerType::Identity,
                ),
                create_column_with_data_and_transfromer_type(
                    "last_name",
                    DataType::PotentialPii,
                    TransformerType::Identity,
                ),
            ]
            .into_iter(),
        );

        println!("{:?}", strategies);

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.person", "last_name"),
        ]);

        let result = validate(&strategies, columns_from_db);

        let error = result.unwrap_err();
        println!("{:?}", error);
        assert_eq!(
            error.unanonymised_pii.unwrap(),
            vec!(
                create_simple_column("public.person", "first_name"),
                create_simple_column("public.person", "last_name")
            )
        );
    }

    fn create_strategy<I>(table_name: &str, columns: I) -> Strategies
    where
        I: Iterator<Item = (String, ColumnInfo)>,
    {
        return HashMap::from([(table_name.to_string(), HashMap::from_iter(columns))]);
    }

    fn add_table<I>(strategies: &mut Strategies, table_name: &str, columns: I)
    where
        I: Iterator<Item = (String, ColumnInfo)>,
    {
        strategies.insert(table_name.to_string(), HashMap::from_iter(columns));
    }

    fn create_column(column_name: &str) -> (String, ColumnInfo) {
        return create_column_with_data_and_transfromer_type(
            column_name,
            DataType::General,
            TransformerType::Identity,
        );
    }

    fn create_column_with_transformer_type(
        column_name: &str,
        transformer_type: TransformerType,
    ) -> (String, ColumnInfo) {
        return create_column_with_data_and_transfromer_type(
            column_name,
            DataType::General,
            transformer_type,
        );
    }

    fn create_strategy_with_data_type(
        column_name: &str,
        data_type: DataType,
    ) -> (String, ColumnInfo) {
        return create_column_with_data_and_transfromer_type(
            column_name,
            data_type,
            TransformerType::Identity,
        );
    }
    fn create_column_with_data_and_transfromer_type(
        column_name: &str,
        data_type: DataType,
        transformer_type: TransformerType,
    ) -> (String, ColumnInfo) {
        return (
            column_name.to_string(),
            ColumnInfo {
                data_type: data_type,
                transformer: Transformer {
                    name: transformer_type,
                    args: None,
                },
            },
        );
    }

    fn create_simple_column(table_name: &str, column_name: &str) -> SimpleColumn {
        return SimpleColumn {
            table_name: table_name.to_string(),
            column_name: column_name.to_string(),
        };
    }
}
