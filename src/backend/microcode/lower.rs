use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter::empty;
use std::rc::Rc;

use itertools::Itertools;

use super::instructions::LoweredInstruction;
use crate::backend::lower_func::lower;
use crate::backend::microcode::instructions::lowered_insts;
use crate::ir::{
    FullBlock, JumpInstruction, Phi, SSAFunction, SSAInstruction, SSAJumpInstruction,
    VirtualRegisterLValue,
};
use crate::utils::rcequality::RcDereferencable;

enum RegisterUse {
    Memory,
    Writeback,
    Mixed,
}

pub fn gen_lowered_blocks(
    func: SSAFunction,
) -> Vec<Rc<RefCell<FullBlock<LoweredInstruction, VirtualRegisterLValue>>>> {
    let mut input_cnt = 0;
    lower(
        func,
        |func, _, inst| lowered_insts(func, inst, &mut input_cnt),
        |_, block_lookup, jmp| {
            (
                vec![],
                match jmp {
                    SSAJumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                        JumpInstruction::BranchIfElseZero {
                            pred,
                            conseq: block_lookup[&conseq.as_key()].clone(),
                            alt: block_lookup[&alt.as_key()].clone(),
                        }
                    }
                    SSAJumpInstruction::Ret(val) => JumpInstruction::Ret(val),
                    SSAJumpInstruction::UnconditionalJump { dest } => {
                        JumpInstruction::UnconditionalJump {
                            dest: block_lookup[&dest.as_key()].clone(),
                        }
                    }
                },
            )
        },
        |lvalue| lvalue,
        |rvalue| rvalue,
    )
    .blocks()
    .collect_vec()
}
