use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    pub classifications: Vec<String>,
}

impl ClassificationConfig {
    pub fn default() -> Self {
        ClassificationConfig {
            classifications: Vec::new(),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read classifications file: {}", e))?;

        serde_json::from_str::<ClassificationConfig>(&content)
            .map_err(|e| format!("Failed to parse classifications JSON: {}", e))
    }

    pub fn is_valid_classification(&self, classification: &str) -> bool {
        self.classifications.iter().any(|c| c == classification)
    }

    /// Returns a HashSet of all classifications, including both built-in and custom classifications.
    ///
    /// This method is currently unused but kept for potential future features such as:
    /// - Displaying all available classifications in a help command
    /// - Supporting validation across both built-in and custom types
    /// - Migration tools that need to understand all possible classifications
    #[allow(dead_code)]
    pub fn get_all_classifications(&self) -> HashSet<String> {
        // Include built-in classifications
        let mut all = HashSet::from([
            "CommerciallySensitive".to_string(),
            "General".to_string(),
            "PotentialPii".to_string(),
            "Pii".to_string(),
            "Security".to_string(),
        ]);

        // Add custom classifications
        all.extend(self.classifications.iter().cloned());

        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_classification() {
        // Create a config with some custom classifications
        let config = ClassificationConfig {
            classifications: vec!["CustomType1".to_string(), "CustomType2".to_string()],
        };

        // Test valid classifications
        assert!(config.is_valid_classification("CustomType1"));
        assert!(config.is_valid_classification("CustomType2"));

        // Test invalid classifications
        assert!(!config.is_valid_classification("CustomType3"));
        assert!(!config.is_valid_classification(""));
    }

    #[test]
    fn test_get_all_classifications() {
        // Create a config with custom classifications
        let config = ClassificationConfig {
            classifications: vec!["CustomType1".to_string(), "CustomType2".to_string()],
        };

        let all = config.get_all_classifications();

        // Should contain built-in classifications
        assert!(all.contains("CommerciallySensitive"));
        assert!(all.contains("General"));
        assert!(all.contains("PotentialPii"));
        assert!(all.contains("Pii"));
        assert!(all.contains("Security"));

        // Should contain custom classifications
        assert!(all.contains("CustomType1"));
        assert!(all.contains("CustomType2"));
    }
}
