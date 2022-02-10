use anyhow::Result;

use self::dominance::{
    dominance_frontiers, find_immediate_dominators, find_immediately_dominated,
    sort_blocks_postorder,
};
use self::gen::gen_expr;
pub use self::instructions::JumpInstruction;
use self::instructions::{Instruction, InstructionRHS};
use self::ssa_transform::{
    alloc_ssa_blocks, backfill_ssa_phis, defining_blocks_for_variables, populate_ssa_blocks,
    ssa_phis,
};
use self::structs::{Block, Function, VirtualVariable};
pub use self::structs::{FullBlock, Phi, VirtualRegister, VirtualRegisterLValue};
use crate::semantics::{FuncDefinition, Program};
use crate::utils::frame::Frame;

mod dominance;
mod gen;
mod instructions;
mod ssa_transform;
mod structs;

pub type SSABlock = FullBlock<SSAInstruction>;
pub type SSAPhi = Phi<SSAInstruction>;
pub type SSAFunction = Function<VirtualRegisterLValue, SSABlock>;
pub type SSAInstruction = Instruction<VirtualRegisterLValue>;
pub type SSAInstructionRHS = InstructionRHS<VirtualRegister>;
pub type SSAJumpInstruction = JumpInstruction<VirtualRegister, SSABlock>;

pub fn gen_ir(program: &Program<FuncDefinition>) -> Result<Program<SSAFunction>> {
    let funcs = program
        .funcs
        .iter()
        .map(|(func_name, func_def)| {
            let mut frame = Frame::new();
            let mut func: Function<VirtualVariable, Block> = Function::new();

            let start_block = func.new_block();

            for arg in func_def.args.iter() {
                let reg = func.new_reg();
                frame.assoc(arg.clone(), reg);
                start_block
                    .borrow_mut()
                    .instructions
                    .push(Instruction::new(reg, InstructionRHS::ReadInput))
            }

            gen_expr(
                &func_def.body,
                &mut func,
                &mut frame,
                &mut vec![],
                start_block.clone(),
            )?;

            // println!("{}", func);

            let (sorted_blocks, index_lookup, predecessors) =
                sort_blocks_postorder(start_block.clone());

            let dominators = find_immediate_dominators(
                start_block.clone(),
                &sorted_blocks,
                &index_lookup,
                &predecessors,
            );
            let dominated = find_immediately_dominated(&sorted_blocks, &dominators);
            let frontiers = dominance_frontiers(&sorted_blocks, &predecessors, &dominators);

            // println!(
            //     "{}\n",
            //     dominators
            //         .iter()
            //         .map(|(k, v)| {
            //             format!(
            //                 "dominator[{}] = {}",
            //                 k.get_ref().borrow().debug_index,
            //                 v.borrow().debug_index
            //             )
            //         })
            //         .join("\n")
            // );

            // println!(
            //     "{}\n",
            //     frontiers
            //         .iter()
            //         .map(|(k, v)| {
            //             format!(
            //                 "frontiers[{}] = [{}]",
            //                 k.get_ref().borrow().debug_index,
            //                 v.iter().map(|b| b.borrow().debug_index).join(", ")
            //             )
            //         })
            //         .join("\n")
            // );

            let variable_defns = defining_blocks_for_variables(&sorted_blocks);

            // println!(
            //     "{}\n",
            //     variable_defns
            //         .iter()
            //         .map(|(k, v)| {
            //             format!(
            //                 "{} defined in blocks [{}]",
            //                 k,
            //                 v.iter()
            //                     .map(|block| block.get_ref().borrow().debug_index)
            //                     .join(", ")
            //             )
            //         })
            //         .join("\n")
            // );

            let mut func = Function::new();
            let phis = ssa_phis(&mut func, &variable_defns, &frontiers);

            // println!(
            //     "{}\n",
            //     phis.iter()
            //         .map(|(k, v)| {
            //             format!(
            //                 "phis[{}] = [{}]",
            //                 k.get_ref().borrow().debug_index,
            //                 v.iter().map(|(a, b)| format!("{a} -> {b}")).join(", ")
            //             )
            //         })
            //         .join("\n")
            // );

            let mut blocks = sorted_blocks;
            blocks.reverse();

            let ssa_blocks = alloc_ssa_blocks(&mut func, &blocks);

            let (ssa_frames, ssa_phi_vars) =
                populate_ssa_blocks(&mut func, start_block, phis, &dominated, &ssa_blocks);
            backfill_ssa_phis(&blocks, &ssa_blocks, &ssa_frames, &ssa_phi_vars);

            Ok((func_name.to_string(), func))
        })
        .collect::<Result<_>>()?;

    Ok(Program { funcs })
}
