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

    pub fn from_strategies_in_file(
        strategies_in_file: Vec<StrategyInFile>,
        transformer_overrides: &TransformerOverrides,
    ) -> Strategies {
        let mut transformed_strategies = Strategies::new();
        //TODO If all columns are none, lets not do any transforming?
        for strategy in strategies_in_file {
            let columns = strategy
                .columns
                .into_iter()
                .map(|column| {
                    (
                        column.name.clone(),
                        ColumnInfo {
                            data_category: column.data_category.clone(),
                            name: column.name.clone(),
                            transformer: transformer(column, &transformer_overrides),
                        },
                    )
                })
                .collect();

            transformed_strategies.insert(strategy.table_name, columns);
        }

        transformed_strategies
    }

    pub fn for_table(&self, table_name: &str) -> Option<&HashMap<String, ColumnInfo>> {
        self.tables.get(table_name)
    }

    pub fn insert(&mut self, table_name: String, columns: HashMap<String, ColumnInfo>) {
        match self.tables.insert(table_name.clone(), columns) {
            None => (),
            Some(_existing) => panic!(

                "Duplicate table {:?} found in strategy file! try running `check-strategies` with --fix", table_name
            ),
        }
    }

    pub fn validate(&self, columns_from_db: HashSet<SimpleColumn>) -> Result<(), MissingColumns> {
        let mut errors = MissingColumns::new();
        for (table_name, columns) in &self.tables {
            for (column_name, column_info) in columns {
                if (column_info.data_category == DataCategory::PotentialPii
                    || column_info.data_category == DataCategory::Pii)
                    && column_info.transformer.name == TransformerType::Identity
                {
                    errors
                        .unanonymised_pii
                        .push(create_simple_column(&column_name, &table_name));
                }
                if column_info.data_category == DataCategory::Unknown {
                    errors
                        .unknown_data_categories
                        .push(create_simple_column(&column_name, &table_name));
                }
                if column_info.transformer.name == TransformerType::Error {
                    errors
                        .error_transformer_types
                        .push(create_simple_column(&column_name, &table_name));
                }
            }
        }

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

        errors.missing_from_strategy_file = add_if_present(in_db_but_not_strategy_file);
        errors.missing_from_db = add_if_present(in_strategy_file_but_not_db);

        if MissingColumns::is_empty(&errors) {
            Ok(())
        } else {
            // TODO i wanted to do like errors.sort() and errors.is_empty()
            // above but couldnt work out the ownership :(
            errors.missing_from_strategy_file.sort();
            errors.missing_from_db.sort();
            errors.unknown_data_categories.sort();
            errors.error_transformer_types.sort();
            errors.unanonymised_pii.sort();
            Err(errors)
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

fn column_transformer_is_overriden(
    data_category: DataCategory,
    overrides: &TransformerOverrides,
) -> bool {
    (data_category == DataCategory::PotentialPii && overrides.allow_potential_pii)
        || (data_category == DataCategory::CommerciallySensitive
            && overrides.allow_commercially_sensitive)
}
fn transformer(column: ColumnInFile, overrides: &TransformerOverrides) -> Transformer {
    if column_transformer_is_overriden(column.data_category, overrides) {
        Transformer {
            name: TransformerType::Identity,
            args: None,
        }
    } else {
        column.transformer
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

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.person", "last_name"),
        ]);

        let result = strategies.validate(columns_from_db);

        let error = result.unwrap_err();

        assert_eq!(
            error.unanonymised_pii,
            vec!(
                create_simple_column("public.person", "first_name"),
                create_simple_column("public.person", "last_name")
            )
        );
    }

    const TABLE_NAME: &str = "gert_lush_table";
    const PII_COLUMN_NAME: &str = "pii_column";
    const COMMERCIALLY_SENSITIVE_COLUMN_NAME: &str = "commercially_sensitive_column";

    #[test]
    fn can_parse_file_contents_into_hashmaps() {
        let column_name = "column1";

        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            columns: vec![column_in_file(
                DataCategory::Pii,
                column_name,
                TransformerType::Scramble,
            )],
        }];

        let expected = Strategies::new_from(
            TABLE_NAME.to_string(),
            HashMap::from([(
                column_name.to_string(),
                ColumnInfo::builder()
                    .with_name(column_name)
                    .with_data_category(DataCategory::Pii)
                    .with_transformer(TransformerType::Scramble, None)
                    .build(),
            )]),
        );
        let parsed = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none());
        assert_eq!(expected, parsed);
    }

    #[test]
    fn ignores_transformers_for_potential_pii_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            columns: vec![
                column_in_file(
                    DataCategory::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataCategory::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides {
                allow_potential_pii: true,
                allow_commercially_sensitive: false,
            },
        );
        let pii_column_transformer = transformer_for_column(PII_COLUMN_NAME, &parsed);
        let commercially_sensitive_transformer =
            transformer_for_column(COMMERCIALLY_SENSITIVE_COLUMN_NAME, &parsed);

        assert_eq!(pii_column_transformer.name, TransformerType::Identity);
        assert_eq!(pii_column_transformer.args, None);

        assert_eq!(
            commercially_sensitive_transformer.name,
            TransformerType::Scramble
        );
        assert_eq!(commercially_sensitive_transformer.args, None);
    }

    #[test]
    fn ignores_transformers_for_commercially_sensitive_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            columns: vec![
                column_in_file(
                    DataCategory::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataCategory::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides {
                allow_potential_pii: false,
                allow_commercially_sensitive: true,
            },
        );

        let commercially_sensitive_transformer =
            transformer_for_column(COMMERCIALLY_SENSITIVE_COLUMN_NAME, &parsed);
        let pii_column_transformer = transformer_for_column(PII_COLUMN_NAME, &parsed);

        assert_eq!(
            commercially_sensitive_transformer.name,
            TransformerType::Identity
        );
        assert_eq!(commercially_sensitive_transformer.args, None);

        assert_eq!(pii_column_transformer.name, TransformerType::Scramble);
        assert_eq!(pii_column_transformer.args, None);
    }

    #[test]
    fn can_combine_override_flags() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            columns: vec![
                column_in_file(
                    DataCategory::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataCategory::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides {
                allow_potential_pii: true,
                allow_commercially_sensitive: true,
            },
        );

        let commercially_sensitive_transformer =
            transformer_for_column(COMMERCIALLY_SENSITIVE_COLUMN_NAME, &parsed);
        let pii_column_transformer = transformer_for_column(PII_COLUMN_NAME, &parsed);

        assert_eq!(
            commercially_sensitive_transformer.name,
            TransformerType::Identity
        );
        assert_eq!(pii_column_transformer.name, TransformerType::Identity);
    }

    fn transformer_for_column(column_name: &str, strategies: &Strategies) -> Transformer {
        strategies
            .transformer_for_column(TABLE_NAME, column_name)
            .expect("expecting a transformer!")
    }

    fn column_in_file(
        data_category: DataCategory,
        name: &str,
        transformer_type: TransformerType,
    ) -> ColumnInFile {
        ColumnInFile {
            data_category,
            description: name.to_string(),
            name: name.to_string(),
            transformer: Transformer {
                name: transformer_type,
                args: None,
            },
        }
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
