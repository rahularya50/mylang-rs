use anyhow::Result;

use self::dominance::{find_immediate_dominators, sort_blocks_postorder};
use self::gen::gen_expr;
use self::instructions::{Instruction, JumpInstruction};
use self::structs::Function;
pub use self::structs::{SSABlock, VirtualRegister, VirtualRegisterLValue};
use crate::ir::dominance::{dominance_frontiers, find_immediately_dominated};
use crate::ir::ssa_transform::{
    alloc_ssa_blocks, backfill_ssa_phis, defining_blocks_for_variables, populate_ssa_blocks,
    ssa_phis,
};
use crate::semantics::Expr;
use crate::utils::frame::Frame;

mod dominance;
mod gen;
mod instructions;
mod ssa_transform;
mod structs;

pub type SSAFunction = Function<VirtualRegisterLValue, SSABlock>;
pub type SSAInstruction = Instruction<VirtualRegisterLValue>;
pub type SSAJumpInstruction = JumpInstruction<VirtualRegister, SSABlock>;

pub fn gen_ssa(expr: &mut Expr) -> Result<Function<VirtualRegisterLValue, SSABlock>> {
    let mut func = Function::new();
    let start_block = func.new_block();
    gen_expr(
        expr,
        &mut func,
        &mut Frame::new(),
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

    let mut blocks = sorted_blocks;
    blocks.reverse();

    let ssa_blocks = alloc_ssa_blocks(&mut func, &blocks);

    let (ssa_frames, ssa_phi_vars) =
        populate_ssa_blocks(&mut func, start_block, phis, &dominated, &ssa_blocks);
    backfill_ssa_phis(&blocks, &ssa_blocks, &ssa_frames, &ssa_phi_vars);

    Ok(func)
}
