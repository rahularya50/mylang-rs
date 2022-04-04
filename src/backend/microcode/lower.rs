use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use super::instructions::LoweredInstruction;
use crate::backend::microcode::instructions::lowered_insts;
use crate::ir::{FullBlock, JumpInstruction, Phi, SSAFunction, SSAJumpInstruction};
use crate::utils::rcequality::RcDereferencable;

enum RegisterUse {
    Memory,
    Writeback,
    Mixed,
}

pub fn gen_lowered_blocks(
    mut func: SSAFunction,
) -> impl IntoIterator<Item = Rc<RefCell<FullBlock<LoweredInstruction>>>> {
    let mut block_lookup = HashMap::new();
    for block in func.blocks() {
        block_lookup.insert(block.as_key(), Rc::new(RefCell::new(FullBlock::default())));
    }
    let mut input_cnt = 0;
    for block_ref in func.blocks().collect_vec() {
        let out_block = block_lookup.get(&block_ref.as_key()).unwrap();
        let block = block_ref.take();
        let mut instructions = vec![];
        for inst in block.instructions {
            instructions.extend(lowered_insts(&mut func, inst, &mut input_cnt))
        }
        out_block.borrow_mut().debug_index = block.debug_index;
        out_block.borrow_mut().preds = block
            .preds
            .into_iter()
            .filter_map(|pred| pred.get_ref().upgrade())
            .map(|pred| Rc::downgrade(&block_lookup[&pred.as_key()]).into())
            .collect();
        out_block.borrow_mut().phis = block
            .phis
            .into_iter()
            .map(|phi| Phi {
                srcs: phi
                    .srcs
                    .into_iter()
                    .map(|(k, v)| (Rc::downgrade(&block_lookup[k.borrow()]).into(), v))
                    .collect(),
                dest: phi.dest,
            })
            .collect();
        out_block.borrow_mut().instructions = instructions;
        out_block.borrow_mut().exit = match block.exit {
            SSAJumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                JumpInstruction::BranchIfElseZero {
                    pred,
                    conseq: block_lookup[&conseq.as_key()].clone(),
                    alt: block_lookup[&alt.as_key()].clone(),
                }
            }
            SSAJumpInstruction::Ret(val) => JumpInstruction::Ret(val),
            SSAJumpInstruction::UnconditionalJump { dest } => JumpInstruction::UnconditionalJump {
                dest: block_lookup[&dest.as_key()].clone(),
            },
        };
    }
    block_lookup.into_values()
}
