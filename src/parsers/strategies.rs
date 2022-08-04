use crate::parsers::strategy_structs::*;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub struct Strategies {
    tables: HashMap<String, HashMap<String, ColumnInfo>>,
}
impl Strategies {
    pub fn new() -> Strategies {
        Strategies {
            tables: HashMap::new(),
        }
    }

    pub fn for_table(&self, table_name: &str) -> Option<&HashMap<String, ColumnInfo>> {
        self.tables.get(table_name)
    }

    pub fn insert(&mut self, table_name: String, columns: HashMap<String, ColumnInfo>) {
        self.tables.insert(table_name, columns);
    }

    pub fn validate(&self, columns_from_db: HashSet<SimpleColumn>) -> Result<(), MissingColumns> {
        //TODO probably dont iterate over and over again!

        let unanonymised_pii: Vec<SimpleColumn> = self
            .tables
            .iter()
            .flat_map(|(table_name, columns)| {
                return columns
                    .iter()
                    .filter(|(_, column_info)| {
                        (column_info.data_category == DataCategory::PotentialPii
                            || column_info.data_category == DataCategory::Pii)
                            && column_info.transformer.name == TransformerType::Identity
                    })
                    .map(|(column_name, _)| create_simple_column(column_name, table_name));
            })
            .collect();
        let unknown_data_categories: Vec<SimpleColumn> = self
            .tables
            .iter()
            .flat_map(|(table_name, columns)| {
                return columns
                    .iter()
                    .filter(|(_, column_info)| column_info.data_category == DataCategory::Unknown)
                    .map(|(column_name, _)| create_simple_column(column_name, table_name));
            })
            .collect();

        let error_transformer_types: Vec<SimpleColumn> = self
            .tables
            .iter()
            .flat_map(|(table_name, columns)| {
                return columns
                    .iter()
                    .filter(|(_, column_info)| {
                        column_info.transformer.name == TransformerType::Error
                    })
                    .map(|(column_name, _)| create_simple_column(column_name, table_name));
            })
            .collect();

        let columns_from_strategy_file: HashSet<SimpleColumn> = self
            .tables
            .iter()
            .flat_map(|(table, columns)| {
                return columns
                    .iter()
                    .map(|(column, _)| create_simple_column(column, table));
            })
            .collect();

        let in_strategy_file_but_not_db: Vec<_> = columns_from_strategy_file
            .difference(&columns_from_db)
            .cloned()
            .collect();

        let in_db_but_not_strategy_file: Vec<_> = columns_from_db
            .difference(&columns_from_strategy_file)
            .cloned()
            .collect();

        if in_db_but_not_strategy_file.is_empty()
            && in_strategy_file_but_not_db.is_empty()
            && unknown_data_categories.is_empty()
            && error_transformer_types.is_empty()
            && unanonymised_pii.is_empty()
        {
            Ok(())
        } else {
            Err(MissingColumns {
                missing_from_db: add_if_present(in_strategy_file_but_not_db),
                missing_from_strategy_file: add_if_present(in_db_but_not_strategy_file),
                unknown_data_categories: add_if_present(unknown_data_categories),
                error_transformer_types: add_if_present(error_transformer_types),
                unanonymised_pii: add_if_present(unanonymised_pii),
            })
        }
    }

    #[allow(dead_code)] //This is used in tests for convenience
    pub fn transformer_for_column<'a>(
        &self,
        table_name: &'a str,
        column_name: &'a str,
    ) -> Option<Transformer> {
        self.tables
            .get(table_name)
            .and_then(|table| table.get(column_name))
            .map(|column| column.transformer.clone())
    }

    #[allow(dead_code)] //This is used in tests for convenience
    pub fn new_from(table_name: String, columns: HashMap<String, ColumnInfo>) -> Strategies {
        Strategies {
            tables: HashMap::from([(table_name, columns)]),
        }
    }
}

fn create_simple_column(column_name: &str, table_name: &str) -> SimpleColumn {
    SimpleColumn {
        table_name: table_name.to_string(),
        column_name: column_name.to_string(),
    }
}

fn add_if_present(list: Vec<SimpleColumn>) -> Vec<SimpleColumn> {
    if list.is_empty() {
        list
    } else {
        let mut new_list = list;
        new_list.sort();
        new_list
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, TransformerType};
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

        let result = strategies.validate(columns_from_db);

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

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_db.is_empty());
        assert_eq!(
            error.missing_from_strategy_file,
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

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_strategy_file.is_empty());
        assert_eq!(
            error.missing_from_db,
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    #[test]
    fn returns_fields_missing_both() {
        let strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        let columns_from_db = HashSet::from([create_simple_column("public.location", "postcode")]);

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.missing_from_strategy_file,
            vec!(create_simple_column("public.location", "postcode"))
        );
        assert_eq!(
            error.missing_from_db,
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn returns_columns_missing_data_category() {
        let strategies = create_strategy(
            "public.person",
            [create_strategy_with_data_category(
                "first_name",
                DataCategory::Unknown,
            )]
            .into_iter(),
        );

        let columns_from_db = HashSet::from([create_simple_column("public.person", "first_name")]);

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.unknown_data_categories,
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

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        assert_eq!(
            error.error_transformer_types,
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn returns_pii_columns_with_identity_transformer() {
        let strategies = create_strategy(
            "public.person",
            [
                create_column_with_data_and_transformer_type(
                    "first_name",
                    DataCategory::Pii,
                    TransformerType::Identity,
                ),
                create_column_with_data_and_transformer_type(
                    "last_name",
                    DataCategory::PotentialPii,
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

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();
        println!("{:?}", error);
        assert_eq!(
            error.unanonymised_pii,
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
        let mut strategies = Strategies::new();
        strategies.insert(table_name.to_string(), HashMap::from_iter(columns));
        strategies
    }

    fn add_table<I>(strategies: &mut Strategies, table_name: &str, columns: I)
    where
        I: Iterator<Item = (String, ColumnInfo)>,
    {
        strategies.insert(table_name.to_string(), HashMap::from_iter(columns));
    }

    fn create_column(column_name: &str) -> (String, ColumnInfo) {
        create_column_with_data_and_transformer_type(
            column_name,
            DataCategory::General,
            TransformerType::Identity,
        )
    }

    fn create_column_with_transformer_type(
        column_name: &str,
        transformer_type: TransformerType,
    ) -> (String, ColumnInfo) {
        create_column_with_data_and_transformer_type(
            column_name,
            DataCategory::General,
            transformer_type,
        )
    }

    fn create_strategy_with_data_category(
        column_name: &str,
        data_category: DataCategory,
    ) -> (String, ColumnInfo) {
        create_column_with_data_and_transformer_type(
            column_name,
            data_category,
            TransformerType::Identity,
        )
    }
    fn create_column_with_data_and_transformer_type(
        column_name: &str,
        data_category: DataCategory,
        transformer_type: TransformerType,
    ) -> (String, ColumnInfo) {
        (
            column_name.to_string(),
            ColumnInfo::builder()
                .with_name(column_name)
                .with_data_category(data_category)
                .with_transformer(transformer_type, None)
                .build(),
        )
    }

    fn create_simple_column(table_name: &str, column_name: &str) -> SimpleColumn {
        SimpleColumn {
            table_name: table_name.to_string(),
            column_name: column_name.to_string(),
        }
    }
}
