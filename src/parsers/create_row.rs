use crate::parsers::sanitiser;
pub fn is_create_row(line: &str) -> bool {
    (line.starts_with("CREATE TABLE ") || line.starts_with("CREATE UNLOGGED TABLE "))
        && line.ends_with('(')
}
pub fn parse(line: &str) -> String {
    let result = line
        .strip_prefix("CREATE TABLE ")
        .or_else(|| line.strip_prefix("CREATE UNLOGGED TABLE "))
        .and_then(|s| s.strip_suffix(" ("));

    match result {
        None => panic!("Create table string doesn't look right??? \"{}\"", line),
        Some(name) => sanitiser::dequote_column_or_table_name_data(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn is_create_row_identifies_CREATE_TABLE() {
        let create_row = "CREATE TABLE public.candidate_details (";
        assert!(is_create_row(create_row))
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_create_row_identifies_CREATE_UNLOGGED_TABLE() {
        let create_row = "CREATE UNLOGGED TABLE public.candidate_details (";
        assert!(is_create_row(create_row))
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_create_row_skips_column_containing_CREATE_TABLE() {
        let not_a_create_row = "CREATE TABLE abc(id uuid);";
        assert!(!is_create_row(not_a_create_row))
    }

    #[test]
    fn create_table_row_is_and_parsed() {
        let create_row = "CREATE TABLE public.candidate_details (";
        let table_name = parse(create_row);
        assert_eq!(table_name, "public.candidate_details".to_string());
    }

    #[test]
    #[should_panic(expected = "Create table string doesn't look right")]
    fn panics_on_invalid_input() {
        let create_row = "CREATE NOTHING public.candidate_details (";
        parse(create_row);
    }
}
