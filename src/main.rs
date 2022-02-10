#![feature(drain_filter)]
#![feature(let_else)]

use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;

use crate::backend::microcode::gen_microops;
use crate::frontend::parse;
use crate::ir::gen_ir;
use crate::optimizations::optimize;
use crate::semantics::analyze;

mod backend;
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
    target: PathBuf,
    #[clap(short, long)]
    fold_constants: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let contents = read_to_string(args.target).expect("unable to open source file");
    let exprs = parse(&mut contents.chars())?;
    let program = analyze(&exprs)?;

    let mut program = gen_ir(&program)?;
    // don't do constant folding for microcode output, since constants are expensive
    optimize(&mut program, args.fold_constants);

    let microops = gen_microops(
        program
            .funcs
            .remove("main")
            .expect("main() function must be defined"),
    )
    .into_iter()
    .collect_vec();

    for block in microops {
        println!("{}", block.borrow());
    }
    Ok(())
}
