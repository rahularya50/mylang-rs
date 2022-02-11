use std::collections::HashMap;
use std::iter::empty;

use itertools::Itertools;

use self::lower::gen_lowered_blocks;
use super::register_liveness::find_liveness;
use crate::ir::{SSAFunction, WithRegisters};

mod instructions;
mod lower;

pub fn lower_to_microcode(func: SSAFunction) {
    let lowered_blocks = gen_lowered_blocks(func).into_iter().collect_vec();
    let registers = lowered_blocks
        .iter()
        .flat_map(|block| {
            empty()
                .chain(
                    block
                        .borrow()
                        .phis
                        .iter()
                        .flat_map(|phi| phi.srcs.values())
                        .cloned(),
                )
                .chain(
                    block
                        .borrow()
                        .instructions
                        .iter()
                        .flat_map(|inst| inst.regs())
                        .cloned(),
                )
                .collect_vec()
        })
        .map(|reg| (reg, find_liveness(&lowered_blocks, reg)))
        .collect::<HashMap<_, _>>();

    for block in &lowered_blocks {
        println!("{}", block.borrow());
    }

    for (reg, lifetimes) in registers.iter().sorted_by_key(|(reg, _)| reg.to_owned()) {
        println!(
            "{}",
            format!(
                "{reg}:\n{}",
                lifetimes
                    .iter()
                    .sorted_by_key(|(block, _)| block.get_ref().borrow().debug_index)
                    .map(|(block, lifetime)| format!(
                        "\t{}: {:?}",
                        block.get_ref().borrow().debug_index,
                        lifetime
                    ))
                    .join("\n")
            )
        );
    }
}
