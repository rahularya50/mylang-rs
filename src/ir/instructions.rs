use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;
use std::rc::Rc;

use super::structs::{BlockWithDebugIndex, RegisterLValue};
use crate::semantics::Operator;
use crate::utils::frame::Frame;
use crate::utils::rcequality::{RcEquality, RcEqualityKey};

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
    ReadInput,
}

impl<RegType: Eq + Hash + Copy> InstructionRHS<RegType> {
    pub fn map_reg_types<NewRegType: Copy>(
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
            InstructionRHS::ReadInput => InstructionRHS::ReadInput,
        })
    }

    pub fn regs(&self) -> impl Iterator<Item = &RegType> {
        (match self {
            InstructionRHS::ArithmeticOperation {
                operator: _,
                arg1,
                arg2,
            } => vec![arg1, arg2],
            InstructionRHS::LoadIntegerLiteral { value: _ } => vec![],
            InstructionRHS::Move { src } => vec![src],
            InstructionRHS::ReadInput => vec![],
        })
        .into_iter()
    }

    pub fn regs_mut(&mut self) -> impl Iterator<Item = &mut RegType> {
        (match self {
            InstructionRHS::ArithmeticOperation {
                operator: _,
                arg1,
                arg2,
            } => vec![arg1, arg2],
            InstructionRHS::LoadIntegerLiteral { value: _ } => vec![],
            InstructionRHS::Move { src } => vec![src],
            InstructionRHS::ReadInput => vec![],
        })
        .into_iter()
    }
}

#[derive(Debug)]
pub struct Instruction<LValue: RegisterLValue> {
    pub lhs: LValue,
    pub rhs: InstructionRHS<LValue::RValue>,
}

impl<LValue: RegisterLValue> Instruction<LValue> {
    pub fn new(lhs: LValue, rhs: InstructionRHS<LValue::RValue>) -> Self {
        Self { lhs, rhs }
    }
}

impl<LValue: RegisterLValue + Display> Display for Instruction<LValue>
where
    LValue::RValue: Display,
{
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
            InstructionRHS::ReadInput => {
                write!(f, "input()")
            }
        }
    }
}

#[derive(Debug)]
pub enum JumpInstruction<RegType, BlockType> {
    BranchIfElseZero {
        pred: RegType,
        conseq: Rc<RefCell<BlockType>>,
        alt: Rc<RefCell<BlockType>>,
    },
    UnconditionalJump {
        dest: Rc<RefCell<BlockType>>,
    },
    Ret(Option<RegType>),
}

impl<RegType, BlockType> JumpInstruction<RegType, BlockType> {
    pub fn srcs(&self) -> impl Iterator<Item = &RegType> {
        (match self {
            JumpInstruction::BranchIfElseZero { pred, .. } => {
                vec![pred]
            }
            JumpInstruction::UnconditionalJump { dest: _ } | JumpInstruction::Ret(None) => vec![],
            JumpInstruction::Ret(Some(out)) => vec![out],
        })
        .into_iter()
    }

    pub fn srcs_mut(&mut self) -> impl Iterator<Item = &mut RegType> {
        (match self {
            JumpInstruction::BranchIfElseZero { pred, .. } => {
                vec![pred]
            }
            JumpInstruction::UnconditionalJump { dest: _ } | JumpInstruction::Ret(None) => vec![],
            JumpInstruction::Ret(Some(out)) => vec![out],
        })
        .into_iter()
    }

    pub fn dests(&self) -> impl Iterator<Item = &Rc<RefCell<BlockType>>> {
        (match self {
            JumpInstruction::BranchIfElseZero { conseq, alt, .. } => {
                vec![conseq, alt]
            }
            JumpInstruction::UnconditionalJump { dest } => vec![dest],
            JumpInstruction::Ret(_) => vec![],
        })
        .into_iter()
    }

    pub fn dests_mut(&mut self) -> impl Iterator<Item = &mut Rc<RefCell<BlockType>>> {
        (match self {
            JumpInstruction::BranchIfElseZero { conseq, alt, .. } => {
                vec![conseq, alt]
            }
            JumpInstruction::UnconditionalJump { dest } => vec![dest],
            JumpInstruction::Ret(_) => vec![],
        })
        .into_iter()
    }

    pub fn map_reg_block_types<NewRegType: Copy, NewBlockType>(
        &self,
        frame: &Frame<RegType, NewRegType>,
        block_lookup: &HashMap<RcEquality<Rc<RefCell<BlockType>>>, Rc<RefCell<NewBlockType>>>,
    ) -> Option<JumpInstruction<NewRegType, NewBlockType>>
    where
        RegType: Hash + Eq,
    {
        Some(match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                JumpInstruction::BranchIfElseZero {
                    pred: frame.lookup(pred)?,
                    conseq: block_lookup.get(&conseq.as_key())?.clone(),
                    alt: block_lookup.get(&alt.as_key())?.clone(),
                }
            }
            JumpInstruction::UnconditionalJump { dest } => JumpInstruction::UnconditionalJump {
                dest: block_lookup.get(&dest.as_key())?.clone(),
            },
            JumpInstruction::Ret(val) => JumpInstruction::Ret(match val {
                Some(val) => Some(frame.lookup(val)?),
                None => None,
            }),
        })
    }
}

impl<RegType: Display, BlockType: BlockWithDebugIndex> Display
    for JumpInstruction<RegType, BlockType>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                write!(
                    f,
                    "if {}==0 branchto {} else {}",
                    pred,
                    conseq.borrow().get_debug_index(),
                    alt.borrow().get_debug_index(),
                )
            }
            JumpInstruction::UnconditionalJump { dest } => {
                write!(f, "jumpto {}", dest.borrow().get_debug_index())
            }
            JumpInstruction::Ret(val) => match val {
                Some(val) => write!(f, "ret {}", val),
                None => write!(f, "ret"),
            },
        }
    }
}
