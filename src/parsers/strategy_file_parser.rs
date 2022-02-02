use crate::parsers::strategy_structs::*;
use std::collections::HashMap;

pub fn parse(
    strategies: Vec<StrategyInFile>,
    transformer_overrides: TransformerOverrides,
) -> HashMap<String, HashMap<String, ColumnInfo>> {
    let mut transformed_strategies: HashMap<String, HashMap<String, ColumnInfo>> = HashMap::new();
    //TODO If all columns are none, lets not do any transforming?
    for strategy in strategies {
        let columns = strategy
            .columns
            .into_iter()
            .map(|column| {
                return (
                    column.name.clone(),
                    ColumnInfo {
                        data_type: column.data_type.clone(),
                        transformer: transformer(column, &transformer_overrides),
                    },
                );
            })
            .collect();

        transformed_strategies.insert(strategy.table_name, columns);
    }

    return transformed_strategies;
}

fn transformer(column: ColumnInFile, overrides: &TransformerOverrides) -> Transformer {
    if column.data_type == DataType::PotentialPii && overrides.allow_potential_pii {
        return Transformer {
            name: TransformerType::Identity,
            args: None,
        };
    } else if column.data_type == DataType::CommerciallySensitive
        && overrides.allow_commercially_sensitive
    {
        return Transformer {
            name: TransformerType::Identity,
            args: None,
        };
    } else {
        return column.transformer;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                DataType::Pii,
                column_name,
                TransformerType::Scramble,
            )],
        }];

        let expected = HashMap::from([(
            TABLE_NAME.to_string(),
            HashMap::from([(
                column_name.to_string(),
                ColumnInfo {
                    data_type: DataType::Pii,
                    transformer: Transformer {
                        name: TransformerType::Scramble,
                        args: None,
                    },
                },
            )]),
        )]);
        let parsed = parse(strategies, TransformerOverrides::default());
        assert_eq!(expected, parsed);
    }

    #[test]
    fn ignores_transformers_for_potential_pii_if_flag_provided() {
        let strategies = vec![StrategyInFile {
            table_name: TABLE_NAME.to_string(),
            description: "description".to_string(),
            columns: vec![
                column_in_file(
                    DataType::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataType::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = parse(
            strategies,
            TransformerOverrides {
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
                    DataType::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataType::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = parse(
            strategies,
            TransformerOverrides {
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
                    DataType::PotentialPii,
                    PII_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
                column_in_file(
                    DataType::CommerciallySensitive,
                    COMMERCIALLY_SENSITIVE_COLUMN_NAME,
                    TransformerType::Scramble,
                ),
            ],
        }];

        let parsed = parse(
            strategies,
            TransformerOverrides {
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

    fn transformer_for_column(
        column_name: &str,
        strategies: &HashMap<String, HashMap<String, ColumnInfo>>,
    ) -> Transformer {
        return strategies[TABLE_NAME][column_name].transformer.clone();
    }

    fn column_in_file(
        data_type: DataType,
        name: &str,
        transformer_type: TransformerType,
    ) -> ColumnInFile {
        return ColumnInFile {
            data_type: data_type,
            description: name.to_string(),
            name: name.to_string(),
            transformer: Transformer {
                name: transformer_type,
                args: None,
            },
        };
    }
}
