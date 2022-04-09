use std::collections::HashMap;
use std::iter::empty;

use itertools::Itertools;

use self::lower::lower_func;
use super::register_coloring::{build_register_graph, color_registers};
use super::register_liveness::find_liveness;
use crate::ir::SSAFunction;

mod instructions;
mod lower;

pub fn lower_to_microcode(func: SSAFunction) {
    let lowered_func = lower_func(func);
    let register_lifetimes = lowered_func
        .blocks()
        .flat_map(|block| {
            empty()
                .chain(block.borrow().phis.iter().map(|phi| phi.dest.0))
                .chain(block.borrow().instructions.iter().map(|inst| inst.lhs.0))
                .collect_vec()
        })
        .map(|reg| (reg, find_liveness(&lowered_func, reg)))
        .collect::<HashMap<_, _>>();

    for block in lowered_func.blocks() {
        println!("{}", block.borrow());
    }

    let register_conflicts = build_register_graph(&register_lifetimes);
    let _register_allocation = color_registers(&register_conflicts, 2);
    // lower(func, |func, , map_jump, map_lvalues, map_rvalues)
}
