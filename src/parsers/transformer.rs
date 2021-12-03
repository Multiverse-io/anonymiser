use crate::strategy_file::Transformer;
use crate::strategy_file::TransformerType;

pub fn transform<'line, 'state>(value: &'line str, transform: &Transformer) -> &'line str {
    match transform.name {
        TransformerType::Identity => value,
        TransformerType::Test => "TestData",
        _ => panic!("unhandled transform: {:?}", transform),
    }
}
