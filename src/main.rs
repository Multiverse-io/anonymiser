mod file_reader;
mod parsers;
use parsers::strategy_file;
fn main() {
    let strategies = strategy_file::parse("new_mappings.json");
    file_reader::read(&strategies);
}
