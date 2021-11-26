pub fn parse(line: String) {
    if line.starts_with("COPY ") {
        //TODO we want to start store some state here to say we're in the copy
        print!("{:?}", line);
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
