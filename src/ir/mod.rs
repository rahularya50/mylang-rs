use anyhow::Result;

use self::dominance::{find_immediate_dominators, sort_blocks_postorder};
use self::gen::gen_expr;
use self::structs::{Function, SSABlock, VirtualRegisterLValue};
use crate::ir::dominance::{dominance_frontiers, find_immediately_dominated};
use crate::ir::ssa_transform::{
    alloc_ssa_blocks, defining_blocks_for_variables, populate_ssa_blocks, ssa_phis,
};
use crate::semantics::Expr;
use crate::utils::Frame;

mod dominance;
mod gen;
mod instructions;
mod ssa_transform;
mod structs;

pub fn gen_ssa(expr: &mut Expr) -> Result<Function<VirtualRegisterLValue, SSABlock>> {
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
    let dominators = find_immediate_dominators(
        start_block.clone(),
        &sorted_blocks,
        &index_lookup,
        &predecessors,
    );
    let dominated = find_immediately_dominated(&sorted_blocks, &dominators);
    let frontiers = dominance_frontiers(&sorted_blocks, &predecessors, &dominators);

    let variable_defns = defining_blocks_for_variables(&sorted_blocks);

    let mut func = Function::new();
    let phis = ssa_phis(&mut func, &variable_defns, &frontiers);

    let ssa_blocks = alloc_ssa_blocks(&mut func, &sorted_blocks);
    let ssa_frames = populate_ssa_blocks(&mut func, start_block, phis, &dominated, &ssa_blocks);

    println!("{func}");

    // TODO: actually bring it into SSA form!
    Ok(func)
}
