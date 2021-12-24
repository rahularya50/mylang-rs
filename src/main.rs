mod lexer;
mod parser;
mod semantics;
mod ssa_gen;

use std::fs::read_to_string;

use anyhow::{Context, Result};
use parser::parse;
use semantics::analyze;
use ssa_gen::gen_ssa;

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;

    let exprs = parse(&mut contents.chars())?;

    // for expr in exprs.iter() {
    //     println!("{}", expr);
    // }

    let func = analyze(&exprs)?;
    // println!("{:#?}", func);

    let ssa = gen_ssa(&func)?;
    println!("{}", ssa);
    // println!("{:#?}", ssa);
    // for block in ssa.blocks.iter() {
    //     println!("{:#?}", block.borrow());
    // }

    Ok(())
}
