mod lexer;
mod parser;
mod semantics;

use std::fs::read_to_string;

use anyhow::{Context, Result};
use parser::parse;
use semantics::analyze;

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;

    let exprs = parse(&mut contents.chars())?;

    for expr in exprs.iter() {
        println!("{}", expr);
    }

    println!("{:#?}", analyze(&exprs)?);

    Ok(())
}
