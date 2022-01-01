#![feature(drain_filter)]
use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::frontend::parse;
use crate::ir::gen_ssa;
use crate::optimizations::optimize;
use crate::semantics::analyze;

mod frontend;
mod ir;
mod optimizations;
mod semantics;
mod utils;

#[derive(Parser)]
#[clap(about, version, author)]
struct Args {
    /// The file to compile
    #[clap(short, long, required = true)]
    target: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let contents = read_to_string(args.target.unwrap()).context("unable to open source file")?;
    let exprs = parse(&mut contents.chars())?;
    let mut func = analyze(&exprs)?;

    let mut ssa = gen_ssa(&mut func)?;
    optimize(&mut ssa);

    println!("{}", ssa);
    Ok(())
}
