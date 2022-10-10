#[cfg(test)]
pub mod builders {
    use crate::parsers::state::Types;
    use crate::parsers::strategy_structs::{
        ColumnInFile, ColumnInfo, DataCategory, StrategyInFile, Transformer, TransformerType,
    };
    use crate::parsers::types::{SubType, Type};
    use std::collections::HashMap;

    impl ColumnInfo {
        pub fn builder() -> ColumnInfoBuilder {
            ColumnInfoBuilder::default()
        }
    }

    #[derive(Default)]
    pub struct ColumnInfoBuilder {
        name: String,
        data_category: Option<DataCategory>,
        transformer_type: Option<TransformerType>,
        transformer_args: Option<HashMap<String, String>>,
    }

    impl ColumnInfoBuilder {
        pub fn with_name(mut self, name: &str) -> ColumnInfoBuilder {
            self.name = name.to_string();
            self
        }

        pub fn with_transformer(
            mut self,
            transformer_type: TransformerType,
            transformer_args: Option<HashMap<String, String>>,
        ) -> ColumnInfoBuilder {
            self.transformer_type = Some(transformer_type);
            self.transformer_args = transformer_args;
            self
        }

        pub fn with_data_category(mut self, data_category: DataCategory) -> ColumnInfoBuilder {
            self.data_category = Some(data_category);
            self
        }

        pub fn build(self) -> ColumnInfo {
            ColumnInfo {
                name: self.name,
                data_category: self.data_category.unwrap_or(DataCategory::General),
                transformer: Transformer {
                    args: self.transformer_args,
                    name: self.transformer_type.unwrap_or(TransformerType::Identity),
                },
            }
        }
    }

    #[derive(Default)]
    pub struct StrategyInFileBuilder {
        table_name: String,
        description: Option<String>,
        columns: Vec<ColumnInFile>,
    }

    impl StrategyInFile {
        pub fn builder() -> StrategyInFileBuilder {
            StrategyInFileBuilder::default()
        }
    }

    impl StrategyInFileBuilder {
        pub fn with_table_name(mut self, name: &str) -> StrategyInFileBuilder {
            self.table_name = name.to_string();
            self
        }

        pub fn with_description(mut self, description: &str) -> StrategyInFileBuilder {
            self.description = Some(description.to_string());
            self
        }

        pub fn with_column(mut self, column: ColumnInFile) -> StrategyInFileBuilder {
            self.columns.push(column);
            self
        }

        pub fn build(self) -> StrategyInFile {
            StrategyInFile {
                table_name: self.table_name,
                description: self.description.unwrap_or("Any description".to_string()),
                columns: self.columns,
            }
        }
    }
    #[derive(Default)]
    pub struct ColumnInFileBuilder {
        name: String,
        description: Option<String>,
        data_category: Option<DataCategory>,
        transformer_type: Option<TransformerType>,
        transformer_args: Option<HashMap<String, String>>,
    }

    impl ColumnInFile {
        pub fn builder() -> ColumnInFileBuilder {
            ColumnInFileBuilder::default()
        }
    }

    impl ColumnInFileBuilder {
        pub fn with_name(mut self, name: &str) -> ColumnInFileBuilder {
            self.name = name.to_string();
            self
        }
        pub fn with_transformer(
            mut self,
            transformer_type: TransformerType,
            transformer_args: Option<HashMap<String, String>>,
        ) -> ColumnInFileBuilder {
            self.transformer_type = Some(transformer_type);
            self.transformer_args = transformer_args;
            self
        }

        pub fn with_data_category(mut self, data_category: DataCategory) -> ColumnInFileBuilder {
            self.data_category = Some(data_category);
            self
        }

        pub fn with_description(mut self, description: &str) -> ColumnInFileBuilder {
            self.description = Some(description.to_string());
            self
        }

        pub fn build(self) -> ColumnInFile {
            ColumnInFile {
                name: self.name,
                data_category: self.data_category.unwrap_or(DataCategory::General),
                description: self.description.unwrap_or("Any description".to_string()),
                transformer: Transformer {
                    args: self.transformer_args,
                    name: self.transformer_type.unwrap_or(TransformerType::Identity),
                },
            }
        }
    }

    impl Types {
        pub fn builder() -> TypesBuilder {
            TypesBuilder::default()
        }
    }

    #[derive(Default)]
    pub struct TypesBuilder {
        types: HashMap<String, HashMap<String, Type>>,
    }

    impl TypesBuilder {
        pub fn add_type(
            self,
            table_name: &str,
            column_name: &str,
            column_type: SubType,
        ) -> TypesBuilder {
            let column_type = Type::SingleValue {
                sub_type: column_type,
            };
            self.common_add_type(table_name, column_name, column_type)
        }

        pub fn add_array_type(
            self,
            table_name: &str,
            column_name: &str,
            array_type: SubType,
        ) -> TypesBuilder {
            let column_type = Type::Array {
                sub_type: array_type,
            };
            self.common_add_type(table_name, column_name, column_type)
        }

        fn common_add_type(
            mut self,
            table_name: &str,
            column_name: &str,
            column_type: Type,
        ) -> TypesBuilder {
            if let Some(existing_table) = self.types.get_mut(table_name) {
                existing_table.insert(column_name.to_string(), column_type);
            } else {
                self.types.insert(
                    table_name.to_string(),
                    HashMap::from_iter([(column_name.to_string(), column_type)]),
                );
            }

            self
        }

        pub fn build(self) -> Types {
            Types::new(self.types)
        }
    }
}
