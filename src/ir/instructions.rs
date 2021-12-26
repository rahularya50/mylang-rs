
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;

use super::structs::{BlockRef, VirtualVariable};
use crate::semantics::Operator;
use crate::utils::Frame;

#[derive(Debug)]
pub enum InstructionRHS<RegType> {
    ArithmeticOperation {
        operator: Operator,
        arg1: RegType,
        arg2: RegType,
    },
    LoadIntegerLiteral {
        value: i64,
    },
    Move {
        src: RegType,
    },
}

impl<RegType: Eq + Hash + Copy> InstructionRHS<RegType> {
    pub fn replace_regs<NewRegType: Copy>(
        &self,
        frame: &Frame<RegType, NewRegType>,
    ) -> Option<InstructionRHS<NewRegType>> {
        Some(match *self {
            InstructionRHS::ArithmeticOperation {
                operator,
                arg1,
                arg2,
            } => InstructionRHS::ArithmeticOperation {
                operator,
                arg1: frame.lookup(&arg1)?,
                arg2: frame.lookup(&arg2)?,
            },
            InstructionRHS::LoadIntegerLiteral { value } => {
                InstructionRHS::LoadIntegerLiteral { value }
            }
            InstructionRHS::Move { src } => InstructionRHS::Move {
                src: frame.lookup(&src)?,
            },
        })
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub lhs: VirtualVariable,
    pub rhs: InstructionRHS<VirtualVariable>,
}

impl Instruction {
    pub fn new(lhs: VirtualVariable, rhs: InstructionRHS<VirtualVariable>) -> Self {
        Instruction { lhs, rhs }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} = ", self.lhs)?;
        match &self.rhs {
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
pub enum JumpInstruction<RegType> {
    BranchIfElseZero {
        pred: RegType,
        conseq: BlockRef,
        alt: BlockRef,
    },
    UnconditionalJump {
        dest: BlockRef,
    },
    Ret,
}

impl<RegType> JumpInstruction<RegType> {
    pub fn dests(&self) -> Vec<BlockRef> {
        match self {
            JumpInstruction::BranchIfElseZero { conseq, alt, .. } => {
                vec![conseq.clone(), alt.clone()]
            }
            JumpInstruction::UnconditionalJump { dest } => vec![dest.clone()],
            JumpInstruction::Ret => vec![],
        }
    }
}

impl<RegType: Display> Display for JumpInstruction<RegType> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                write!(
                    f,
                    "if {}==0 branchto {} else {}",
                    pred,
                    conseq.borrow().debug_index,
                    alt.borrow().debug_index
                )
            }
            JumpInstruction::UnconditionalJump { dest } => {
                write!(f, "jumpto {}", dest.borrow().debug_index)
            }
            JumpInstruction::Ret => write!(f, "ret"),
        }
    }
}
