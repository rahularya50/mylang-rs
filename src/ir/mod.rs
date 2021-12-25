use anyhow::Result;
use itertools::Itertools;

use self::core_structs::Function;
use self::dominance::{find_immediate_dominators, sort_blocks_postorder};
use self::gen::{gen_expr, Frame};
use crate::ir::dominance::dominance_frontiers;
use crate::semantics::Expr;

mod core_structs;
mod dominance;
mod gen;
mod instructions;

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
    let frontiers = dominance_frontiers(&sorted_blocks, &predecessors, &dominators);

    println!(
        "{}",
        sorted_blocks
            .iter()
            .map(|block| {
                (
                    block.borrow().debug_index,
                    frontiers
                        .get(&block.clone().into())
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|dom| dom.borrow().debug_index)
                        .join(","),
                )
            })
            .map(|(a, b)| format!("{a} has frontiers: [{b}]"))
            .join("\n")
    );

    // TODO: actually bring it into SSA form!
    Ok(func)
}
