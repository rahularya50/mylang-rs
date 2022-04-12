use std::cell::RefCell;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;
use std::rc::Rc;

use super::ssa_forms::CfgConfig;
use super::structs::{BlockWithDebugIndex, WithRegisters};
use crate::semantics::{BinaryOperator, UnaryOperator};
use crate::utils::frame::Frame;

#[derive(Debug)]
pub enum InstructionRHS<RegType> {
    ReadMemory(RegType),
    UnaryOperation {
        operator: UnaryOperator,
        arg: RegType,
    },
    BinaryOperation {
        operator: BinaryOperator,
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
            InstructionRHS::ReadMemory(arg) => InstructionRHS::ReadMemory(frame.lookup(&arg)?),
            InstructionRHS::UnaryOperation { operator, arg } => InstructionRHS::UnaryOperation {
                operator,
                arg: frame.lookup(&arg)?,
            },
            InstructionRHS::BinaryOperation {
                operator,
                arg1,
                arg2,
            } => InstructionRHS::BinaryOperation {
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
}

impl<RegType> WithRegisters<RegType> for InstructionRHS<RegType> {
    fn regs(&self) -> <Vec<&RegType> as IntoIterator>::IntoIter {
        (match self {
            InstructionRHS::ReadMemory(arg) => vec![arg],
            InstructionRHS::UnaryOperation { operator: _, arg } => vec![arg],
            InstructionRHS::BinaryOperation {
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

    fn regs_mut(&mut self) -> <Vec<&mut RegType> as IntoIterator>::IntoIter {
        (match self {
            InstructionRHS::ReadMemory(arg) => vec![arg],
            InstructionRHS::UnaryOperation { operator: _, arg } => vec![arg],
            InstructionRHS::BinaryOperation {
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
pub struct Instruction<Conf: CfgConfig> {
    pub lhs: Conf::LValue,
    pub rhs: Conf::RHSType,
}

impl<Conf: CfgConfig> Instruction<Conf> {
    pub fn new(lhs: Conf::LValue, rhs: Conf::RHSType) -> Self {
        Self { lhs, rhs }
    }
}

impl<Conf: CfgConfig> Display for Instruction<Conf>
where
    Conf::LValue: Display,
    Conf::RHSType: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.lhs, self.rhs)
    }
}

impl<T: Display> Display for InstructionRHS<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstructionRHS::ReadMemory(arg) => {
                write!(f, "read {arg}")
            }
            InstructionRHS::UnaryOperation { operator, arg } => {
                write!(f, "{operator:?} {arg}")
            }
            InstructionRHS::BinaryOperation {
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
pub enum JumpInstruction<Conf: CfgConfig> {
    BranchIfElseZero {
        pred: Conf::RValue,
        conseq: Rc<RefCell<Conf::BlockType>>,
        alt: Rc<RefCell<Conf::BlockType>>,
    },
    UnconditionalJump {
        dest: Rc<RefCell<Conf::BlockType>>,
    },
    Ret(Option<Conf::RValue>),
}

impl<Conf: CfgConfig> JumpInstruction<Conf> {
    pub fn dests(&self) -> impl Iterator<Item = &Rc<RefCell<Conf::BlockType>>> {
        (match self {
            JumpInstruction::BranchIfElseZero { conseq, alt, .. } => {
                vec![conseq, alt]
            }
            JumpInstruction::UnconditionalJump { dest } => vec![dest],
            JumpInstruction::Ret(_) => vec![],
        })
        .into_iter()
    }

    pub fn dests_mut(&mut self) -> impl Iterator<Item = &mut Rc<RefCell<Conf::BlockType>>> {
        (match self {
            JumpInstruction::BranchIfElseZero { conseq, alt, .. } => {
                vec![conseq, alt]
            }
            JumpInstruction::UnconditionalJump { dest } => vec![dest],
            JumpInstruction::Ret(_) => vec![],
        })
        .into_iter()
    }

    pub fn map_reg_block_types<NewConf: CfgConfig>(
        &self,
        mut reg_mapper: impl FnMut(&Conf::RValue) -> Option<NewConf::RValue>,
        mut block_mapper: impl FnMut(
            &Rc<RefCell<Conf::BlockType>>,
        ) -> Option<Rc<RefCell<NewConf::BlockType>>>,
    ) -> Option<JumpInstruction<NewConf>>
    where
        Conf::RValue: Hash,
        NewConf::LValue: Hash + Eq,
    {
        Some(match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                JumpInstruction::BranchIfElseZero {
                    pred: reg_mapper(pred)?,
                    conseq: block_mapper(&conseq)?,
                    alt: block_mapper(&alt)?,
                }
            }
            JumpInstruction::UnconditionalJump { dest } => JumpInstruction::UnconditionalJump {
                dest: block_mapper(&dest)?,
            },
            JumpInstruction::Ret(val) => JumpInstruction::Ret(match val {
                Some(val) => Some(reg_mapper(val)?),
                None => None,
            }),
        })
    }
}

impl<Conf: CfgConfig> WithRegisters<Conf::RValue> for JumpInstruction<Conf> {
    fn regs(&self) -> <Vec<&Conf::RValue> as IntoIterator>::IntoIter {
        (match self {
            JumpInstruction::BranchIfElseZero { pred, .. } => {
                vec![pred]
            }
            JumpInstruction::UnconditionalJump { dest: _ } | JumpInstruction::Ret(None) => vec![],
            JumpInstruction::Ret(Some(out)) => vec![out],
        })
        .into_iter()
    }

    fn regs_mut(&mut self) -> <Vec<&mut Conf::RValue> as IntoIterator>::IntoIter {
        (match self {
            JumpInstruction::BranchIfElseZero { pred, .. } => {
                vec![pred]
            }
            JumpInstruction::UnconditionalJump { dest: _ } | JumpInstruction::Ret(None) => vec![],
            JumpInstruction::Ret(Some(out)) => vec![out],
        })
        .into_iter()
    }
}

impl<Conf: CfgConfig> Display for JumpInstruction<Conf>
where
    Conf::RValue: Display,
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
