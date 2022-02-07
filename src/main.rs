#![feature(drain_filter)]
#![feature(let_else)]

use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::frontend::parse;
use crate::ir::gen_ir;
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
    let program = analyze(&exprs)?;

    let mut program = gen_ir(&program)?;
    optimize(&mut program);

    println!("{}", program);
    Ok(())
}
