use crate::parsers::custom_classifications::ClassificationConfig;
use crate::parsers::strategy_structs::*;
use itertools::sorted;
use itertools::Itertools;
use serde_json;
use std::fs;
use std::io::Write;

pub fn read(file_name: &str) -> Result<Vec<StrategyInFile>, std::io::Error> {
    let result = fs::read_to_string(file_name).map(|file_contents| {
        serde_json::from_str::<Vec<StrategyInFile>>(&file_contents).unwrap_or_else(|e| {
            panic!(
                "Invalid json found in strategy file at '{}': {:#}",
                file_name, e
            )
        })
    });

    match result {
        Ok(_) => result,
        Err(ref err) => match err.kind() {
            std::io::ErrorKind::NotFound => result,
            _ => panic!("Unable to read strategy file at {}: {:?}", file_name, err),
        },
    }
}

pub fn write(file_name: &str, mut new_file_contents: Vec<StrategyInFile>) -> std::io::Result<()> {
    new_file_contents.sort();

    for s in new_file_contents.iter_mut() {
        sort_columns(s)
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file_name)?;

    serde_json::to_writer_pretty(file, &new_file_contents)?;

    Ok(())
}
fn sort_columns(s: &mut StrategyInFile) {
    s.columns.sort_by(|a, b| a.name.cmp(&b.name))
}
pub fn to_csv(
    strategy_file: &str,
    csv_output_file: &str,
    custom_classifications: ClassificationConfig,
) -> std::io::Result<()> {
    let strategies = read(strategy_file)?;
    let p: Vec<String> = strategies
        .iter()
        .flat_map(|strategy| {
            strategy.columns.iter().filter_map(|column| {
                // We want to output rows that are not 'General' or if they are custom and invalid.
                let (include_in_csv, validity_info) = match &column.data_category {
                    // Generally, don't include General unless it's an invalid custom one (edge case, see below)
                    DataCategory::General => (false, String::new()),
                    DataCategory::Custom(category_name) => {
                        if !custom_classifications.is_valid_classification(category_name) {
                            // Include invalid custom categories
                            (
                                true,
                                format!(", [INVALID CUSTOM CLASSIFICATION: {}]", category_name),
                            )
                        } else {
                            // Include valid custom, non-General categories
                            (
                                true,
                                format!(", [VALID CUSTOM CLASSIFICATION: {}]", category_name),
                            )
                        }
                    }
                    // For other built-in, non-General categories (Pii, Unknown, etc.)
                    _ => {
                        // No specific validity_info needed for these built-ins in the CSV as they aren't custom
                        (true, String::new())
                    }
                };

                // Special case: if a category is literally named "General" but is defined in custom_classifications.json
                // and is somehow marked invalid (though is_valid_classification implies it exists in the file if true).
                // The logic above for DataCategory::Custom will handle a custom "General".
                // If it was DataCategory::General (built-in), it won't be included unless forced by an invalid custom rule.

                if include_in_csv {
                    Some(format!(
                        "{}, {}, {:?}{}, {}",
                        strategy.table_name,
                        column.name,
                        column.data_category, // Debug representation of enum
                        validity_info,
                        column.description
                    ))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<String>>();
    let to_write = format!(
        "{}\n{}",
        "table name, column name, data category, description",
        sorted(p).join("\n")
    );

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(csv_output_file)?;
    file.write_all(to_write.as_bytes()).unwrap();

    Ok(())
}
