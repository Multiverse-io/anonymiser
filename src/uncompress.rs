use std::fs::File;
use std::path::PathBuf;

pub fn uncompress(input_file: PathBuf, output_file: Option<PathBuf>) -> Result<(), std::io::Error> {
    let input = File::open(input_file)?;
    match output_file {
        Some(output) => zstd::stream::copy_decode(input, File::create(output)?),
        None => zstd::stream::copy_decode(input, std::io::stdout()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{anonymiser::anonymise, parsers::strategy_structs::TransformerOverrides};

    use super::uncompress;

    #[test]
    fn compress_gives_correct_output() {
        let test_dir_path = PathBuf::from("test_files/compress");
        std::fs::create_dir_all(&test_dir_path).unwrap();

        anonymise(
            "test_files/dump_file.sql".to_string(),
            "test_files/compress/results.sql".to_string(),
            "test_files/strategy.json".to_string(),
            None,
            TransformerOverrides::none(),
        )
        .unwrap();

        anonymise(
            "test_files/dump_file.sql".to_string(),
            "test_files/compress/results.sql.zst".to_string(),
            "test_files/strategy.json".to_string(),
            Some(None),
            TransformerOverrides::none(),
        )
        .unwrap();

        uncompress(
            PathBuf::from("test_files/compress/results.sql.zst"),
            Some(test_dir_path.join("uncompressed.sql")),
        )
        .unwrap();

        // Can't compare actual content because of randomization, but # of lines
        // should be the same
        assert_eq!(
            std::fs::read_to_string("test_files/compress/results.sql")
                .unwrap()
                .lines()
                .count(),
            std::fs::read_to_string("test_files/compress/uncompressed.sql")
                .unwrap()
                .lines()
                .count()
        );

        std::fs::remove_dir_all(test_dir_path).unwrap();
    }
}
