pub fn parse(line : String) {
    if line.starts_with("COPY "){
        crate::parsers::copy_row::parse(line);
    } else if line.starts_with("\\.") {

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        parse("ooh".to_string())
    }
}

