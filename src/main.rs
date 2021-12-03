mod file_reader;
mod parsers;
use parsers::strategy_file;
fn main() -> Result<(), std::io::Error> {
    let strategies = strategy_file::parse("new_mappings.json");
    file_reader::read(&strategies)?;
    return Ok(());
}
