use crate::parsers::row_parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub fn read(strategies: HashMap<String, Vec<String>>) {
    match read_lines("clear_text_dump_big.sql") {
        Ok(lines) => {
            for line in lines {
                if let Ok(ip) = line {
                    row_parser::parse(ip)
                }
            }
        }
        Err(e) => println!("error!! {:?}", e),
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let strategies = HashMap::from([
            ("Mercury".to_string(), vec!["ooh".to_string()]),
            ("Venus".to_string(), vec!["ooh".to_string()]),
        ]);
        let x = read(strategies);
        print!("{:?}", x);
    }
}
