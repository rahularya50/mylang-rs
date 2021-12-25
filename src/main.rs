use std::fs::read_to_string;

use anyhow::{Context, Result};

use crate::frontend::parse;
use crate::semantics::analyze;
use crate::ssa::gen_ssa;

mod frontend;
mod semantics;
mod ssa;

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;

    let exprs = parse(&mut contents.chars())?;

    // for expr in exprs.iter() {
    //     println!("{}", expr);
    // }

    let mut func = analyze(&exprs)?;
    // println!("{:#?}", func);

    let ssa = gen_ssa(&mut func)?;
    println!("{}", ssa);
    // println!("{:#?}", ssa);
    // for block in ssa.blocks.iter() {
    //     println!("{:#?}", block.borrow());
    // }

    Ok(())
}
