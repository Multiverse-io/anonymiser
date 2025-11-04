use crate::parsers::custom_classifications::ClassificationConfig;
use crate::parsers::strategy_errors::{DbErrors, ValidationErrors};
use crate::parsers::strategy_structs::*;
use itertools::{Either, Itertools};
use std::collections::HashMap;
use std::collections::HashSet;

type ColumnNamesToInfo = HashMap<String, ColumnInfo>;

#[derive(Debug, PartialEq, Eq)]
pub struct Strategies {
    tables: HashMap<String, TableStrategy>,
    salt: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TableStrategy {
    Columns(ColumnNamesToInfo),
    Truncate,
}

impl Strategies {
    pub fn new() -> Strategies {
        Strategies {
            tables: HashMap::new(),
            salt: None,
        }
    }

    pub fn from_strategies_in_file(
        strategies_in_file: Vec<StrategyInFile>,
        transformer_overrides: &TransformerOverrides,
        custom_classifications: &ClassificationConfig,
    ) -> Result<Strategies, Box<ValidationErrors>> {
        let mut transformed_strategies = Strategies::new();
        let mut errors = ValidationErrors::new();

        // Check if the first item is a salt configuration
        if let Some(first) = strategies_in_file.first() {
            transformed_strategies.salt = if first.table_name.is_empty() {
                first.salt.clone()
            } else {
                None
            };
        }

        for strategy in strategies_in_file {
            if strategy.table_name.is_empty() {
                continue;
            }

            validate_deterministic_settings(&strategy, &mut errors);

            if strategy.truncate {
                transformed_strategies.insert_truncate(strategy.table_name);
            } else {
                let mut columns = HashMap::<String, ColumnInfo>::new();
                for column in strategy.columns {
                    // Validate custom classifications
                    if let DataCategory::Custom(category_name) = &column.data_category {
                        if !custom_classifications.is_valid_classification(category_name) {
                            errors
                                .invalid_custom_classifications
                                .push(create_simple_column(&strategy.table_name, &column.name));
                        }
                    }
                    // Built-in categories don't need custom validation against the file.

                    if (column.data_category == DataCategory::PotentialPii
                        || column.data_category == DataCategory::Pii)
                        && column.transformer.name == TransformerType::Identity
                    {
                        errors
                            .unanonymised_pii
                            .push(create_simple_column(&strategy.table_name, &column.name));
                    }
                    if column.data_category == DataCategory::Unknown {
                        errors
                            .unknown_data_categories
                            .push(create_simple_column(&strategy.table_name, &column.name));
                    }
                    if column.transformer.name == TransformerType::Error {
                        errors
                            .error_transformer_types
                            .push(create_simple_column(&strategy.table_name, &column.name));
                    }
                    let result = columns.insert(
                        column.name.clone(),
                        ColumnInfo {
                            data_category: column.data_category.clone(),
                            name: column.name.clone(),
                            transformer: transformer(column, transformer_overrides),
                        },
                    );
                    if let Some(dupe) = result {
                        errors.duplicate_columns.push(create_simple_column(
                            &strategy.table_name.clone(),
                            &dupe.name,
                        ))
                    }
                }

                let result = transformed_strategies.insert(strategy.table_name.clone(), columns);
                if result.is_some() {
                    errors.duplicate_tables.push(strategy.table_name);
                }
            }
        }

        if ValidationErrors::is_empty(&errors) {
            Ok(transformed_strategies)
        } else {
            //TODO sort/order errors somehow or maybe only do that when we log them out??
            Err(Box::new(errors))
        }
    }

    pub fn for_table(&self, table_name: &str) -> Option<&TableStrategy> {
        self.tables.get(table_name)
    }

    pub fn insert(
        &mut self,
        table_name: String,
        columns: HashMap<String, ColumnInfo>,
    ) -> Option<TableStrategy> {
        self.tables
            .insert(table_name, TableStrategy::Columns(columns))
    }
    pub fn insert_truncate(&mut self, table_name: String) -> Option<TableStrategy> {
        self.tables.insert(table_name, TableStrategy::Truncate)
    }

    pub fn validate_against_db(
        &self,
        columns_from_db: HashSet<SimpleColumn>,
    ) -> Result<(), DbErrors> {
        let (columns_by_table, truncate): (Vec<(String, ColumnNamesToInfo)>, Vec<_>) = self
            .tables
            .clone()
            .into_iter()
            .partition_map(|(table, table_strategy)| match table_strategy {
                TableStrategy::Columns(columns) => Either::Left((table, columns)),
                TableStrategy::Truncate => Either::Right(table),
            });

        let columns_from_strategy_file: HashSet<SimpleColumn> = columns_by_table
            .iter()
            .flat_map(|(table_name, columns)| {
                columns
                    .keys()
                    .map(|column_name| create_simple_column(table_name, column_name))
            })
            .collect();

        let columns_from_db_without_truncate: HashSet<SimpleColumn> = columns_from_db
            .iter()
            .filter(|column| !truncate.contains(&column.table_name))
            .cloned()
            .collect();

        let mut errors = DbErrors {
            missing_from_strategy_file: columns_from_db_without_truncate
                .difference(&columns_from_strategy_file)
                .cloned()
                .collect(),
            missing_from_db: columns_from_strategy_file
                .difference(&columns_from_db_without_truncate)
                .cloned()
                .collect(),
        };

        if DbErrors::is_empty(&errors) {
            Ok(())
        } else {
            // TODO i wanted to do like errors.sort() and errors.is_empty()
            // above but couldnt work out the ownership :(
            errors.missing_from_strategy_file.sort();
            errors.missing_from_db.sort();
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
            .and_then(|table| match table {
                TableStrategy::Columns(columns) => columns.get(column_name),
                TableStrategy::Truncate => None,
            })
            .map(|column| column.transformer.clone())
    }

    #[allow(dead_code)] //This is used in tests for convenience
    pub fn new_from(table_name: String, columns: HashMap<String, ColumnInfo>) -> Strategies {
        Strategies {
            tables: HashMap::from([(table_name, TableStrategy::Columns(columns))]),
            salt: None,
        }
    }

    #[allow(dead_code)] //This is used in tests for convenience
    pub fn new_from_with_salt(
        table_name: String,
        columns: HashMap<String, ColumnInfo>,
        salt: Option<String>,
    ) -> Strategies {
        Strategies {
            tables: HashMap::from([(table_name, TableStrategy::Columns(columns))]),
            salt,
        }
    }

    pub fn salt_for_table(&self, table_name: &str) -> Option<&str> {
        if self.tables.contains_key(table_name) {
            self.salt.as_deref()
        } else {
            None
        }
    }
}

fn create_simple_column(table_name: &str, column_name: &str) -> SimpleColumn {
    SimpleColumn {
        table_name: table_name.to_string(),
        column_name: column_name.to_string(),
    }
}

fn apply_transformer_overrides(
    data_category: DataCategory,
    overrides: &TransformerOverrides,
    transformer: Transformer,
) -> Transformer {
    match data_category {
        DataCategory::PotentialPii if overrides.allow_potential_pii => Transformer {
            name: TransformerType::Identity,
            args: None,
        },
        DataCategory::CommerciallySensitive if overrides.allow_commercially_sensitive => {
            Transformer {
                name: TransformerType::Identity,
                args: None,
            }
        }
        _ if overrides.scramble_blank && transformer.name == TransformerType::Scramble => {
            Transformer {
                name: TransformerType::ScrambleBlank,
                args: None,
            }
        }
        _ => transformer,
    }
}

fn transformer(column: ColumnInFile, overrides: &TransformerOverrides) -> Transformer {
    apply_transformer_overrides(column.data_category, overrides, column.transformer)
}

/// Validates transformers with deterministic=true have an id_column (except FakeUUID transformer).
fn validate_deterministic_settings(strategy: &StrategyInFile, errors: &mut ValidationErrors) {
    strategy
        .columns
        .iter()
        .filter(|column| column.transformer.name != TransformerType::FakeUUID)
        .filter_map(|column| {
            column.transformer.args.as_ref().and_then(|args| {
                if args.get("deterministic") == Some(&"true".to_string())
                    && !args.contains_key("id_column")
                {
                    Some(create_simple_column(&strategy.table_name, &column.name))
                } else {
                    None
                }
            })
        })
        .for_each(|simple_column| {
            errors.deterministic_without_id.push(simple_column);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::strategy_structs::{ColumnInfo, TransformerType};
    use std::collections::HashMap;

    #[test]
    fn validate_against_db_returns_ok_with_matching_fields() {
        let mut strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        add_table(
            &mut strategies,
            "public.location",
            [create_column("postcode")].into_iter(),
            None,
        );

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.location", "postcode"),
        ]);

        let result = strategies.validate_against_db(columns_from_db);

        assert!(result.is_ok());
    }

    #[test]
    fn validate_against_db_returns_fields_missing_from_strategy_file_that_are_in_the_db() {
        let strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        let columns_from_db = HashSet::from([
            create_simple_column("public.person", "first_name"),
            create_simple_column("public.location", "postcode"),
        ]);

        let result = strategies.validate_against_db(columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_db.is_empty());
        assert_eq!(
            error.missing_from_strategy_file,
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    #[test]
    fn validate_against_db_returns_fields_missing_from_the_db_but_are_in_the_strategy_file() {
        let mut strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        add_table(
            &mut strategies,
            "public.location",
            [create_column("postcode")].into_iter(),
            None,
        );

        let columns_from_db = HashSet::from([create_simple_column("public.person", "first_name")]);

        let result = strategies.validate_against_db(columns_from_db);

        let error = result.unwrap_err();
        assert!(error.missing_from_strategy_file.is_empty());
        assert_eq!(
            error.missing_from_db,
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    #[test]
    fn validate_against_db_returns_fields_missing_both() {
        let strategies =
            create_strategy("public.person", [create_column("first_name")].into_iter());

        let columns_from_db = HashSet::from([create_simple_column("public.location", "postcode")]);

        let result = strategies.validate_against_db(columns_from_db);

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
    fn validates_truncate() {
        let mut strategies = Strategies::new();
        strategies.insert_truncate("public.location".to_string());

        let columns_from_db = HashSet::from([create_simple_column("public.location", "postcode")]);
        let result = strategies.validate_against_db(columns_from_db);

        assert_eq!(Ok(()), result);
    }

    #[test]
    fn validates_missing_entire_table() {
        let strategies = Strategies::new();

        let columns_from_db = HashSet::from([create_simple_column("public.location", "postcode")]);
        let error = strategies.validate_against_db(columns_from_db).unwrap_err();

        assert_eq!(
            error.missing_from_strategy_file,
            vec!(create_simple_column("public.location", "postcode"))
        );
    }

    const TABLE_NAME: &str = "gert_lush_table";
    const PII_COLUMN_NAME: &str = "pii_column";
    const COMMERCIALLY_SENSITIVE_COLUMN_NAME: &str = "commercially_sensitive_column";
    const SCRAMBLED_COLUMN_NAME: &str = "scrambled_column";

    #[test]
    fn from_strategies_in_file_can_parse_file_contents_into_hashmaps() {
        let column_name = "column1";

        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
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
        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");
        assert_eq!(expected, parsed);
    }

    #[test]
    fn from_strategies_in_file_can_parse_file_contents_with_salt_into_hashmaps() {
        let column_name = "column1";
        let salt = "test_salt".to_string();

        let strategies = vec![
            // First item is salt configuration (matches JSON structure)
            StrategyInFile {
                table_name: String::default(), // Will be empty string
                description: String::default(),
                truncate: false,
                salt: Some(salt.clone()),
                columns: Vec::default(),
            },
            // Actual table strategy
            StrategyInFile {
                table_name: TABLE_NAME.to_string(),
                description: "description".to_string(),
                truncate: false,
                salt: None,
                columns: vec![column_in_file(
                    DataCategory::Pii,
                    column_name,
                    TransformerType::Scramble,
                )],
            },
        ];

        let expected = Strategies::new_from_with_salt(
            TABLE_NAME.to_string(),
            HashMap::from([(
                column_name.to_string(),
                ColumnInfo::builder()
                    .with_name(column_name)
                    .with_data_category(DataCategory::Pii)
                    .with_transformer(TransformerType::Scramble, None)
                    .build(),
            )]),
            Some(salt),
        );
        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");
        assert_eq!(expected, parsed);
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_duplicate_table_and_column_definitions() {
        let table2_name = "daps";
        let column_name = "column1";
        let duplicated_column =
            column_in_file(DataCategory::Pii, column_name, TransformerType::Scramble);

        let strategies = vec![
            StrategyInFile {
                table_name: TABLE_NAME.to_string(),
                description: "description".to_string(),
                truncate: false,
                salt: None,
                columns: vec![],
            },
            StrategyInFile {
                table_name: TABLE_NAME.to_string(),
                description: "description".to_string(),
                truncate: false,
                salt: None,
                columns: vec![],
            },
            StrategyInFile {
                table_name: table2_name.to_string(),
                description: "description".to_string(),
                truncate: false,
                salt: None,
                columns: vec![duplicated_column.clone(), duplicated_column],
            },
        ];

        let error = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        )
        .expect_err("We should have a duplicate table error");

        assert_eq!(error.duplicate_tables, vec![TABLE_NAME.to_string()]);
        assert_eq!(
            error.duplicate_columns,
            vec![create_simple_column(table2_name, column_name)]
        );
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_columns_missing_data_category() {
        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![column_in_file(
                DataCategory::Unknown,
                "first_name",
                TransformerType::Identity,
            )],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );

        let error = result.unwrap_err();
        assert_eq!(
            error.unknown_data_categories,
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_columns_with_error_transformer_types() {
        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![column_in_file(
                DataCategory::General,
                "first_name",
                TransformerType::Error,
            )],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );

        let error = result.unwrap_err();
        assert_eq!(
            error.error_transformer_types,
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_pii_columns_with_identity_transformer() {
        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![
                column_in_file(DataCategory::Pii, "first_name", TransformerType::Identity),
                column_in_file(
                    DataCategory::PotentialPii,
                    "last_name",
                    TransformerType::Identity,
                ),
            ],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );

        let error = result.unwrap_err();

        assert_eq!(
            error.unanonymised_pii,
            vec!(
                create_simple_column("public.person", "first_name"),
                create_simple_column("public.person", "last_name")
            )
        );
    }

    #[test]
    fn from_strategies_in_file_ignores_transformers_for_potential_pii_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
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
                ..Default::default()
            },
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");
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
    fn from_strategies_in_file_ignores_transformers_for_commercially_sensitive_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
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
                ..Default::default()
            },
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");

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
    fn from_strategies_in_file_modifies_transformer_for_scramble_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![column_in_file(
                DataCategory::General,
                SCRAMBLED_COLUMN_NAME,
                TransformerType::Scramble,
            )],
        }];

        let parsed = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides {
                scramble_blank: true,
                ..Default::default()
            },
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");

        let scramble_transformer = transformer_for_column(SCRAMBLED_COLUMN_NAME, &parsed);

        assert_eq!(scramble_transformer.name, TransformerType::ScrambleBlank);
    }

    #[test]
    fn from_strategies_in_file_can_combine_override_flags() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
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
                scramble_blank: true,
            },
            &ClassificationConfig::default(),
        )
        .expect("we shouldnt have duplicate columns!");

        // Both of these override scramble_blank

        let commercially_sensitive_transformer =
            transformer_for_column(COMMERCIALLY_SENSITIVE_COLUMN_NAME, &parsed);
        let pii_column_transformer = transformer_for_column(PII_COLUMN_NAME, &parsed);

        assert_eq!(
            commercially_sensitive_transformer.name,
            TransformerType::Identity
        );
        assert_eq!(pii_column_transformer.name, TransformerType::Identity);
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_deterministic_without_id_column() {
        let mut transformer = Transformer {
            name: TransformerType::Scramble,
            args: Some(HashMap::new()),
        };
        transformer
            .args
            .as_mut()
            .unwrap()
            .insert("deterministic".to_string(), "true".to_string());

        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![ColumnInFile {
                data_category: DataCategory::General,
                description: "first_name".to_string(),
                name: "first_name".to_string(),
                transformer,
            }],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );

        let error = result.unwrap_err();
        assert_eq!(
            error.deterministic_without_id,
            vec!(create_simple_column("public.person", "first_name"))
        );
    }

    #[test]
    fn from_strategies_in_file_accepts_deterministic_with_id_column() {
        let mut transformer = Transformer {
            name: TransformerType::Scramble,
            args: Some(HashMap::new()),
        };
        transformer
            .args
            .as_mut()
            .unwrap()
            .insert("deterministic".to_string(), "true".to_string());
        transformer
            .args
            .as_mut()
            .unwrap()
            .insert("id_column".to_string(), "user_id".to_string());

        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![ColumnInFile {
                data_category: DataCategory::General,
                description: "first_name".to_string(),
                name: "first_name".to_string(),
                transformer,
            }],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn from_strategies_in_file_accepts_fake_uuid_with_deterministic_without_id_column() {
        // Set up a FakeUUID transformer with deterministic=true but no id_column
        let mut transformer = Transformer {
            name: TransformerType::FakeUUID,
            args: Some(HashMap::new()),
        };
        transformer
            .args
            .as_mut()
            .unwrap()
            .insert("deterministic".to_string(), "true".to_string());

        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![ColumnInFile {
                data_category: DataCategory::General,
                description: "user_id".to_string(),
                name: "user_id".to_string(),
                transformer,
            }],
        }];

        let result = Strategies::from_strategies_in_file(
            strategies,
            &TransformerOverrides::none(),
            &ClassificationConfig::default(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_invalid_custom_classification() {
        let strategies_in_file = vec![StrategyInFile {
            table_name: "public.table_with_custom".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![column_in_file(
                DataCategory::Custom("InvalidCustomType".to_string()),
                "custom_column",
                TransformerType::Identity,
            )],
        }];

        let custom_classifications = ClassificationConfig {
            classifications: vec!["ValidCustomType".to_string()],
        };

        let result = Strategies::from_strategies_in_file(
            strategies_in_file,
            &TransformerOverrides::none(),
            &custom_classifications,
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_custom_classifications.len(), 1);
        assert_eq!(
            error.invalid_custom_classifications[0],
            create_simple_column("public.table_with_custom", "custom_column")
        );
        // Ensure other error fields are empty
        assert!(error.unknown_data_categories.is_empty());
        assert!(error.error_transformer_types.is_empty());
        assert!(error.unanonymised_pii.is_empty());
        assert!(error.duplicate_columns.is_empty());
        assert!(error.duplicate_tables.is_empty());
        assert!(error.deterministic_without_id.is_empty());
    }

    #[test]
    fn from_strategies_in_file_accepts_valid_custom_classifications() {
        let custom_type = "ValidCustomType";

        let strategies_in_file = vec![StrategyInFile {
            table_name: "public.table_with_custom".to_string(),
            description: "description".to_string(),
            truncate: false,
            salt: None,
            columns: vec![column_in_file(
                DataCategory::Custom(custom_type.to_string()),
                "custom_column",
                TransformerType::Identity,
            )],
        }];

        let custom_classifications = ClassificationConfig {
            classifications: vec![custom_type.to_string()],
        };

        let result = Strategies::from_strategies_in_file(
            strategies_in_file,
            &TransformerOverrides::none(),
            &custom_classifications,
        );

        assert!(result.is_ok());

        // Verify the parsed strategy contains the custom classification
        let strategies = result.unwrap();
        if let Some(TableStrategy::Columns(columns)) =
            strategies.for_table("public.table_with_custom")
        {
            if let Some(column_info) = columns.get("custom_column") {
                match &column_info.data_category {
                    DataCategory::Custom(name) => {
                        assert_eq!(name, custom_type);
                    }
                    _ => panic!(
                        "Expected Custom data category, found {:?}",
                        column_info.data_category
                    ),
                }
            } else {
                panic!("Column not found in parsed strategy");
            }
        } else {
            panic!("Table not found in parsed strategy");
        }
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
        add_table(&mut strategies, table_name, columns, None);
        strategies
    }

    fn add_table<I>(strategies: &mut Strategies, table_name: &str, columns: I, salt: Option<String>)
    where
        I: Iterator<Item = (String, ColumnInfo)>,
    {
        strategies.insert(table_name.to_string(), HashMap::from_iter(columns));
        strategies.salt = salt;
    }

    fn create_column(column_name: &str) -> (String, ColumnInfo) {
        create_column_with_data_and_transformer_type(
            column_name,
            DataCategory::General,
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
}
