use anyhow::Result;
mod ast;
mod formatter;
use formatter::Formatter;
mod parser;
use parser::Parser;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Write};

#[macro_use]
extern crate clap;
use clap::App;

fn main() -> Result<()> {
    let yaml = load_yaml!("main.yml");
    let matches = App::from_yaml(yaml).get_matches();
    if let Some(path) = matches.value_of_os("json_file") {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let result = format(reader)?;
        if matches.is_present("in_place") {
            let mut f = File::create(path)?;
            writeln!(f, "{}", result)?;
        } else {
            println!("{}", result);
        }
    } else {
        let stdin = stdin();
        let reader = stdin.lock();
        let result = format(reader)?;
        println!("{}", result);
    }
    Ok(())
}

fn format<R: BufRead>(reader: R) -> Result<String> {
    let v = Parser::new(reader).parse_value()?;
    Ok(Formatter::new().format(v))
}
