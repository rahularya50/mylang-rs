#![feature(drain_filter)]
use std::fs::read_to_string;

use anyhow::{Context, Result};

use crate::frontend::parse;
use crate::ir::gen_ssa;
use crate::optimizations::optimize;
use crate::semantics::analyze;

mod frontend;
mod ir;
mod optimizations;
mod semantics;
mod utils;

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;
    let exprs = parse(&mut contents.chars())?;
    let mut func = analyze(&exprs)?;
    let mut ssa = gen_ssa(&mut func)?;
    optimize(&mut ssa);

    println!("{}", ssa);
    Ok(())
}
