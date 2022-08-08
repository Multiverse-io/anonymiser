pub fn trim(line: &str) -> &str {
    line.trim()
}

pub fn dequote_column_or_table_name_data(line: &str) -> String {
    line.replace('\"', "")
}
