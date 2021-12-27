#![feature(drain_filter)]
use std::fs::read_to_string;

use anyhow::{Context, Result};

use crate::frontend::parse;
use crate::ir::gen_ssa;
use crate::semantics::analyze;

mod frontend;
mod ir;
mod semantics;
mod utils;

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;

    let exprs = parse(&mut contents.chars())?;

    // for expr in exprs.iter() {
    //     println!("{}", expr);
    // }

    let mut func = analyze(&exprs)?;
    // println!("{:#?}", func);

    let _ssa = gen_ssa(&mut func)?;
    // println!("{}", ssa);
    // println!("{:#?}", ssa);
    // for block in ssa.blocks.iter() {
    //     println!("{:#?}", block.borrow());
    // }

    Ok(())
}
