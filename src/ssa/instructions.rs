use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;

use super::core_structs::{BlockRef, VirtualRegister, VirtualRegisterLValue};
use crate::semantics::Operator;

#[derive(Debug)]
pub enum Instruction {
    ArithmeticOperation {
        operator: Operator,
        arg1: VirtualRegister,
        arg2: VirtualRegister,
        out: VirtualRegisterLValue,
    },
    LoadIntegerLiteral {
        value: i64,
        out: VirtualRegisterLValue,
    },
    Move {
        src: VirtualRegister,
        out: VirtualRegisterLValue,
    },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::ArithmeticOperation {
                operator,
                arg1,
                arg2,
                out,
            } => {
                write!(f, "{out} = {arg1} {operator:?} {arg2}")
            }
            Instruction::LoadIntegerLiteral { value, out } => {
                write!(f, "{out} = {value}")
            }
            Instruction::Move { src, out } => {
                write!(f, "{out} = {src}")
            }
        }
    }
}

#[derive(Debug)]
pub enum JumpInstruction {
    BranchIfElseZero {
        pred: VirtualRegister,
        conseq: Rc<BlockRef>,
        alt: Rc<BlockRef>,
    },
    UnconditionalJump {
        dest: Rc<BlockRef>,
    },
}

impl Display for JumpInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                write!(
                    f,
                    "if {}==0 branchto {} else {}",
                    pred,
                    (***conseq).borrow().debug_index,
                    (***alt).borrow().debug_index
                )
            }
            JumpInstruction::UnconditionalJump { dest } => {
                write!(f, "jumpto {}", (***dest).borrow().debug_index)
            }
        }
    }
}
