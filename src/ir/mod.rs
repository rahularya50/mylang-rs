use std::collections::HashMap;

use anyhow::Result;
use itertools::Itertools;

use self::dominance::{find_immediate_dominators, sort_blocks_postorder};
use self::gen::{gen_expr, Frame};
use self::structs::Function;
use crate::ir::dominance::{dominance_frontiers, find_immediately_dominated};
use crate::ir::ssa_transform::{defining_blocks_for_variables, ssa_phis};
use crate::semantics::Expr;

mod dominance;
mod gen;
mod instructions;
mod ssa_transform;
mod structs;

pub fn gen_ssa(expr: &mut Expr) -> Result<Function> {
    let mut func = Function::new();
    let mut frame = Frame::new();
    let start_block = func.start_block.clone();
    gen_expr(
        expr,
        &mut func,
        &mut frame,
        &mut vec![],
        start_block.clone(),
    )?;

    let (sorted_blocks, index_lookup, predecessors) = sort_blocks_postorder(start_block.clone());
    let dominators =
        find_immediate_dominators(start_block, &sorted_blocks, &index_lookup, &predecessors);
    let dominated = find_immediately_dominated(&sorted_blocks, &dominators);
    let frontiers = dominance_frontiers(&sorted_blocks, &predecessors, &dominators);

    let variable_defns = defining_blocks_for_variables(&sorted_blocks);
    let phis = ssa_phis(&mut func, &variable_defns, &frontiers);

    println!("{func}");

    println!(
        "{}",
        sorted_blocks
            .iter()
            .map(|block| {
                (
                    block.borrow().debug_index,
                    phis.get(&block.clone().into())
                        .unwrap_or(&HashMap::new())
                        .iter()
                        .map(|(var, phi)| format!("{var}: {phi}"))
                        .join(","),
                )
            })
            .map(|(a, b)| format!("block {a} has phis: [{b}]"))
            .join("\n")
    );

    // TODO: actually bring it into SSA form!
    Ok(func)
}
