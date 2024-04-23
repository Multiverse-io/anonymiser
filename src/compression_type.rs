use std::str::FromStr;

#[derive(Debug)]
pub enum CompressionType {
    Zstd,
    Gzip,
}
type ParseError = &'static str;

impl FromStr for CompressionType {
    type Err = ParseError;
    fn from_str(compression_type: &str) -> Result<Self, Self::Err> {
        match compression_type {
            "zstd" => Ok(CompressionType::Zstd),
            "gzip" => Ok(CompressionType::Gzip),
            _ => Err("Could not parse compression type"),
        }
    }
}
