use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;

use super::core_structs::{BlockRef, VirtualRegister, VirtualRegisterLValue};
use crate::semantics::Operator;

#[derive(Debug)]
pub enum InstructionRHS {
    ArithmeticOperation {
        operator: Operator,
        arg1: VirtualRegister,
        arg2: VirtualRegister,
    },
    LoadIntegerLiteral {
        value: i64,
    },
    Move {
        src: VirtualRegister,
    },
}

#[derive(Debug)]
pub struct Instruction {
    pub lhs: VirtualRegister,
    pub rhs: InstructionRHS,
}

impl Instruction {
    pub fn new(lhs: VirtualRegister, rhs: InstructionRHS) -> Self {
        Instruction { lhs, rhs }
    }
}

pub struct SSAInstruction {
    lhs: VirtualRegisterLValue,
    rhs: InstructionRHS,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} = ", self.lhs)?;
        match self.rhs {
            InstructionRHS::ArithmeticOperation {
                operator,
                arg1,
                arg2,
            } => {
                write!(f, "{arg1} {operator:?} {arg2}")
            }
            InstructionRHS::LoadIntegerLiteral { value } => {
                write!(f, "{value}")
            }
            InstructionRHS::Move { src } => {
                write!(f, "{src}")
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
