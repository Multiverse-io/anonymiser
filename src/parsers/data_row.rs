use core::str::Split;
pub fn split(line: &str) -> Split<'_, char> {
    line.strip_suffix('\n').unwrap_or(line).split('\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_split() {
        let line = "1\t2\t3\t4\n";
        let result: Vec<&str> = split(line).collect();
        assert_eq!(result, vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn can_split_with_empty_string_at_end() {
        let line = "1\t2\t3\t\n";
        let result: Vec<&str> = split(line).collect();
        assert_eq!(result, vec!["1", "2", "3", ""]);
    }
}
