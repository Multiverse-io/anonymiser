pub fn trim(line: &str) -> &str {
    line.trim_matches(|c| c == ' ' || c == '\n')
}

pub fn dequote_column_or_table_name_data(line: &str) -> String {
    line.replace('\"', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_leading_whitespace_and_newline() {
        let line = "    1234\n";
        let result = trim(line);
        assert_eq!(result, "1234");
    }

    #[test]
    fn does_not_trim_tabs() {
        let line = "    \t1234\t\n";
        let result = trim(line);
        assert_eq!(result, "\t1234\t");
    }

    #[test]
    fn dequotes_given_string() {
        let line = "\"order\"";
        let result = dequote_column_or_table_name_data(line);
        assert_eq!(result, "order");
    }
}
