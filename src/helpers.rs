use crate::parsers::strategy_structs::{Transformer, TransformerType};
use crate::parsers::transformer;
use crate::parsers::types::Type;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::collections::HashMap;


const HELPER_TABLE_NAME: &str = "anonymiser_helper";

/// Anonymise an email address using the FakeEmail transformer
pub fn anonymise_email(email: &str, global_salt: Option<&str>) -> Result<String, String> {
    // Create a transformer struct
    let transformer = Transformer {
        name: TransformerType::FakeEmail,
        args: None,
    };

    // Create a dummy RNG (will be replaced by transformer's internal RNG for deterministic operations)
    let mut rng = SmallRng::seed_from_u64(0);

    // Use string type as default
    let column_type = Type::SingleValue {
        sub_type: crate::parsers::types::SubType::Character,
    };

    // Call the transformer
    let result = transformer::transform(
        &mut rng,
        email,
        &column_type,
        &transformer,
        HELPER_TABLE_NAME,
        &[],
        global_salt,
    );

    Ok(result.into_owned())
}

/// Anonymise an ID using the specified transformer
pub fn anonymise_id(
    id: &str,
    transformer_type: TransformerType,
    args: Option<HashMap<String, String>>,
    global_salt: Option<&str>,
) -> Result<String, String> {
    // Create a transformer struct
    let transformer = Transformer {
        name: transformer_type,
        args,
    };

    // Create a dummy RNG (will be replaced by transformer's internal RNG for deterministic operations)
    let mut rng = SmallRng::seed_from_u64(0);

    // Use string type as default
    let column_type = Type::SingleValue {
        sub_type: crate::parsers::types::SubType::Character,
    };

    // Call the transformer
    let result = transformer::transform(
        &mut rng,
        id,
        &column_type,
        &transformer,
        HELPER_TABLE_NAME,
        &[],
        global_salt,
    );

    Ok(result.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymise_email() {
        let email = "test@example.com";
        let result = anonymise_email(email, None).unwrap();

        // Should not be the same as input
        assert_ne!(result, email);

        // Should be deterministic
        let result2 = anonymise_email(email, None).unwrap();
        assert_eq!(result, result2);

        // Should contain a hash prefix
        assert!(result.contains('-'));
    }

    #[test]
    fn test_anonymise_id_with_fake_uuid() {
        let id = "12345";
        let mut args = HashMap::new();
        args.insert("deterministic".to_string(), "true".to_string());

        let result = anonymise_id(id, TransformerType::FakeUUID, Some(args), None).unwrap();

        // Should not be the same as input
        assert_ne!(result, id);

        // Should be deterministic
        let mut args2 = HashMap::new();
        args2.insert("deterministic".to_string(), "true".to_string());
        let result2 = anonymise_id(id, TransformerType::FakeUUID, Some(args2), None).unwrap();
        assert_eq!(result, result2);

        // Should be a valid UUID format
        assert!(result.len() == 36); // UUID length with hyphens
    }

    #[test]
    fn test_anonymise_id_with_scramble() {
        let id = "user123";
        let result = anonymise_id(id, TransformerType::Scramble, None, None).unwrap();

        // Should not be the same as input
        assert_ne!(result, id);

        // Should maintain the same length
        assert_eq!(result.len(), id.len());
    }
}
