use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationConfig {
    pub classifications: Vec<String>,
}

impl ClassificationConfig {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read classifications file: {}", e))?;

        serde_json::from_str::<ClassificationConfig>(&content)
            .map_err(|e| format!("Failed to parse classifications JSON: {}", e))
    }

    pub fn is_valid_classification(&self, classification: &str) -> bool {
        self.classifications.iter().any(|c| c == classification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
    fn test_from_file_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"{
            "classifications": [
                "TestCustom1",
                "TestCustom2"
            ]
        }"#;
        temp_file.write_all(content.as_bytes()).unwrap();

        let config = ClassificationConfig::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.classifications, vec!["TestCustom1", "TestCustom2"]);
    }

    #[test]
    fn test_from_file_error_not_found() {
        let result = ClassificationConfig::from_file("non_existent_file.json");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .starts_with("Failed to read classifications file:"));
    }

    #[test]
    fn test_from_file_error_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"{
            "classifications": [
                "TestCustom1",
        }"#; // Invalid JSON
        temp_file.write_all(content.as_bytes()).unwrap();

        let result = ClassificationConfig::from_file(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .starts_with("Failed to parse classifications JSON:"));
    }
}
