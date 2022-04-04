use std::collections::HashMap;
use std::iter::empty;

use itertools::Itertools;

use self::lower::gen_lowered_blocks;
use super::register_coloring::{build_register_graph, color_registers};
use super::register_liveness::find_liveness;
use crate::ir::{FullBlock, Instruction, SSAFunction, VirtualRegisterLValue};

mod instructions;
mod lower;

pub type Block<RValue> = FullBlock<Instruction<VirtualRegisterLValue, RValue>, VirtualRegisterLValue>;

pub fn lower_to_microcode(func: SSAFunction) {
    let lowered_blocks = gen_lowered_blocks(func).into_iter().collect_vec();
    let register_lifetimes = lowered_blocks
        .iter()
        .flat_map(|block| {
            empty()
                .chain(block.borrow().phis.iter().map(|phi| phi.dest.0))
                .chain(block.borrow().instructions.iter().map(|inst| inst.lhs.0))
                .collect_vec()
        })
        .map(|reg| (reg, find_liveness(&lowered_blocks, reg)))
        .collect::<HashMap<_, _>>();

    for block in &lowered_blocks {
        println!("{}", block.borrow());
    }

    let register_conflicts = build_register_graph(&register_lifetimes);
    let register_allocation = color_registers(&register_conflicts, 2);
    // lowered_blocks.into_iter().map(|block| block.borrow_mut());
}
