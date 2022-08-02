pub fn parse(line: &str) -> String {
    let result = line
<<<<<<< HEAD
=======
        .trim()
>>>>>>> read_column_types_from_create_table
        .strip_prefix("CREATE TABLE ")
        .and_then(|s| s.strip_suffix(" ("));

    match result {
        None => panic!("Create table string doesn't look right??? \"{}\"", line),
        Some(name) => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_table_row_is_parsed() {
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
