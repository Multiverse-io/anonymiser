use crate::parsers::strategy_structs::*;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub struct Strategies {
    tables: HashMap<String, HashMap<String, ColumnInfo>>,
}

#[derive(Debug)]
pub enum StrategyFileError {
    ValidationError(ValidationErrors),
    DbMismatchError(DbErrors),
}

impl From<ValidationErrors> for StrategyFileError {
    fn from(err: ValidationErrors) -> Self {
        StrategyFileError::ValidationError(err)
    }
}

impl From<DbErrors> for StrategyFileError {
    fn from(err: DbErrors) -> Self {
        StrategyFileError::DbMismatchError(err)
    }
}

#[derive(Debug)]
pub struct DbErrors {
    pub missing_from_strategy_file: Vec<SimpleColumn>,
    pub missing_from_db: Vec<SimpleColumn>,
}
impl DbErrors {
    pub fn is_empty(to_check: &DbErrors) -> bool {
        to_check.missing_from_strategy_file.is_empty() && to_check.missing_from_db.is_empty()
    }
}

#[derive(Debug)]
pub struct ValidationErrors {
    pub unknown_data_categories: Vec<SimpleColumn>,
    pub error_transformer_types: Vec<SimpleColumn>,
    pub unanonymised_pii: Vec<SimpleColumn>,
    pub duplicate_columns: Vec<(String, String)>,
    pub duplicate_tables: Vec<String>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        ValidationErrors {
            unknown_data_categories: Vec::new(),
            error_transformer_types: Vec::new(),
            unanonymised_pii: Vec::new(),
            duplicate_columns: Vec::new(),
            duplicate_tables: Vec::new(),
        }
    }
    pub fn is_empty(to_check: &ValidationErrors) -> bool {
        to_check.unknown_data_categories.is_empty()
            && to_check.error_transformer_types.is_empty()
            && to_check.unanonymised_pii.is_empty()
            && to_check.duplicate_columns.is_empty()
            && to_check.duplicate_tables.is_empty()
    }
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
    ) -> Result<Strategies, ValidationErrors> {
        let mut transformed_strategies = Strategies::new();
        let mut errors = ValidationErrors::new();

        for strategy in strategies_in_file {
            let mut columns = HashMap::<String, ColumnInfo>::new();
            for column in strategy.columns {
                if (column.data_category == DataCategory::PotentialPii
                    || column.data_category == DataCategory::Pii)
                    && column.transformer.name == TransformerType::Identity
                {
                    errors
                        .unanonymised_pii
                        .push(create_simple_column(&column.name, &strategy.table_name));
                }
                if column.data_category == DataCategory::Unknown {
                    errors
                        .unknown_data_categories
                        .push(create_simple_column(&column.name, &strategy.table_name));
                }
                if column.transformer.name == TransformerType::Error {
                    errors
                        .error_transformer_types
                        .push(create_simple_column(&column.name, &strategy.table_name));
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
                    errors
                        .duplicate_columns
                        .push((strategy.table_name.clone(), dupe.name));
                }
            }

            let result = transformed_strategies.insert(strategy.table_name.clone(), columns);
            if result.is_some() {
                errors.duplicate_tables.push(strategy.table_name);
            }
        }

        if ValidationErrors::is_empty(&errors) {
            Ok(transformed_strategies)
        } else {
            //TODO sort/order errors somehow or maybe only do that when we log them out??
            Err(errors)
        }
    }

    pub fn for_table(&self, table_name: &str) -> Option<&HashMap<String, ColumnInfo>> {
        self.tables.get(table_name)
    }

    pub fn insert(
        &mut self,
        table_name: String,
        columns: HashMap<String, ColumnInfo>,
    ) -> Option<HashMap<String, ColumnInfo>> {
        self.tables.insert(table_name, columns)
    }

    pub fn validate_against_db(
        &self,
        columns_from_db: HashSet<SimpleColumn>,
    ) -> Result<(), DbErrors> {
        let columns_from_strategy_file: HashSet<SimpleColumn> = self
            .tables
            .iter()
            .flat_map(|(table, columns)| {
                return columns
                    .iter()
                    .map(|(column, _)| create_simple_column(column, table));
            })
            .collect();

        let mut errors = DbErrors {
            missing_from_strategy_file: columns_from_db
                .difference(&columns_from_strategy_file)
                .cloned()
                .collect(),
            missing_from_db: columns_from_strategy_file
                .difference(&columns_from_db)
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
    fn validate_against_db_returns_ok_with_matching_fields() {
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

    const TABLE_NAME: &str = "gert_lush_table";
    const PII_COLUMN_NAME: &str = "pii_column";
    const COMMERCIALLY_SENSITIVE_COLUMN_NAME: &str = "commercially_sensitive_column";

    #[test]
    fn from_strategies_in_file_can_parse_file_contents_into_hashmaps() {
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
        let parsed = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none())
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
                columns: vec![],
            },
            StrategyInFile {
                table_name: TABLE_NAME.to_string(),
                description: "description".to_string(),
                columns: vec![],
            },
            StrategyInFile {
                table_name: table2_name.to_string(),
                description: "description".to_string(),
                columns: vec![duplicated_column.clone(), duplicated_column],
            },
        ];

        let error = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none())
            .expect_err("We should have a duplicate table error");

        assert_eq!(error.duplicate_tables, vec![TABLE_NAME.to_string()]);
        assert_eq!(
            error.duplicate_columns,
            vec![(table2_name.to_string(), column_name.to_string())]
        );
    }

    #[test]
    fn from_strategies_in_file_returns_errors_for_columns_missing_data_category() {
        let strategies = vec![StrategyInFile {
            table_name: "public.person".to_string(),
            description: "description".to_string(),
            columns: vec![column_in_file(
                DataCategory::Unknown,
                "first_name",
                TransformerType::Identity,
            )],
        }];

        let result = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none());

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
            columns: vec![column_in_file(
                DataCategory::General,
                "first_name",
                TransformerType::Error,
            )],
        }];

        let result = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none());

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
            columns: vec![
                column_in_file(DataCategory::Pii, "first_name", TransformerType::Identity),
                column_in_file(
                    DataCategory::PotentialPii,
                    "last_name",
                    TransformerType::Identity,
                ),
            ],
        }];

        let result = Strategies::from_strategies_in_file(strategies, &TransformerOverrides::none());

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
    fn from_strategies_in_file_can_combine_override_flags() {
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
        )
        .expect("we shouldnt have duplicate columns!");

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
