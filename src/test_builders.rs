#[cfg(test)]
pub mod builders {
    use crate::parsers::state::Types;
    use crate::parsers::strategy_structs::ColumnInfo;
    use crate::parsers::strategy_structs::DataCategory;
    use crate::parsers::strategy_structs::Transformer;
    use crate::parsers::strategy_structs::TransformerType;
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
                    HashMap::from([(column_name.to_string(), column_type)]),
                );
            }

            self
        }

        pub fn build(self) -> Types {
            Types::new(self.types)
        }
    }
}
