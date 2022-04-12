use std::fmt::{self, Display, Formatter};
use std::mem::Discriminant;

use super::lower::MicrocodeConfig;
use crate::backend::register_coloring::PhysicalRegister;
use crate::ir::{
    CfgConfig, Function, Instruction, SSABlock, SSAInstruction, SSAInstructionRHS,
    VirtualRegisterLValue, WithRegisters,
};
use crate::semantics::{BinaryOperator, UnaryOperator};

pub type LoweredInstruction = Instruction<MicrocodeConfig>;

#[derive(Copy, Clone, Debug)]
pub enum UnaryALUOperator {
    Copy,
    Inc1,
    Inc4,
    Dec1,
    Dec4,
}

#[derive(Copy, Clone, Debug)]
pub enum BinaryALUOperator {
    Add,
    Sub,
    Slt,
    Sltu,
    And,
    Or,
    Xor,
}

#[derive(Debug)]
pub enum LoweredInstructionRHS<RegType> {
    UnaryALU {
        operator: UnaryALUOperator,
        arg: RegType,
    },
    BinaryALU {
        operator: BinaryALUOperator,
        arg1: RegType,
        arg2: RegType,
    },
    LoadOneImmediate,
    LoadMemory(RegType),
    StoreMemory {
        addr: RegType,
        data: RegType,
    },
    LoadRegister(u8),
    StoreRegister {
        index: u8,
        value: RegType,
    },
}

impl<RegType> LoweredInstructionRHS<RegType> {
    pub fn allocate_registers(
        self,
        mut mapper: impl FnMut(RegType) -> PhysicalRegister,
    ) -> LoweredInstructionRHS<PhysicalRegister> {
        match self {
            LoweredInstructionRHS::UnaryALU { operator, arg } => LoweredInstructionRHS::UnaryALU {
                operator,
                arg: mapper(arg),
            },
            LoweredInstructionRHS::BinaryALU {
                operator,
                arg1,
                arg2,
            } => LoweredInstructionRHS::BinaryALU {
                operator,
                arg1: mapper(arg1),
                arg2: mapper(arg2),
            },
            LoweredInstructionRHS::LoadOneImmediate => LoweredInstructionRHS::LoadOneImmediate,
            LoweredInstructionRHS::LoadMemory(reg) => {
                LoweredInstructionRHS::LoadMemory(mapper(reg))
            }
            LoweredInstructionRHS::StoreMemory { addr, data } => {
                LoweredInstructionRHS::StoreMemory {
                    addr: mapper(addr),
                    data: mapper(data),
                }
            }
            LoweredInstructionRHS::LoadRegister(i) => LoweredInstructionRHS::LoadRegister(i),
            LoweredInstructionRHS::StoreRegister { index, value } => {
                LoweredInstructionRHS::StoreRegister {
                    index,
                    value: mapper(value),
                }
            }
        }
    }
}

impl<RegType> WithRegisters<RegType> for LoweredInstructionRHS<RegType> {
    fn regs(&self) -> <Vec<&RegType> as IntoIterator>::IntoIter {
        match self {
            LoweredInstructionRHS::UnaryALU { operator: _, arg } => vec![arg],
            LoweredInstructionRHS::BinaryALU {
                operator: _,
                arg1,
                arg2,
            } => vec![arg1, arg2],
            LoweredInstructionRHS::LoadOneImmediate => vec![],
            LoweredInstructionRHS::LoadMemory(reg) => vec![reg],
            LoweredInstructionRHS::StoreMemory { addr, data } => vec![addr, data],
            LoweredInstructionRHS::LoadRegister(_) => vec![],
            LoweredInstructionRHS::StoreRegister { index: _, value } => vec![value],
        }
        .into_iter()
    }

    fn regs_mut(&mut self) -> <Vec<&mut RegType> as IntoIterator>::IntoIter {
        match self {
            LoweredInstructionRHS::UnaryALU { operator: _, arg } => vec![arg],
            LoweredInstructionRHS::BinaryALU {
                operator: _,
                arg1,
                arg2,
            } => vec![arg1, arg2],
            LoweredInstructionRHS::LoadOneImmediate => vec![],
            LoweredInstructionRHS::LoadMemory(reg) => vec![reg],
            LoweredInstructionRHS::StoreMemory { addr, data } => vec![addr, data],
            LoweredInstructionRHS::LoadRegister(_) => vec![],
            LoweredInstructionRHS::StoreRegister { index: _, value } => vec![value],
        }
        .into_iter()
    }
}

impl<T: Display> Display for LoweredInstructionRHS<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoweredInstructionRHS::LoadMemory(arg) => {
                write!(f, "read {arg}")
            }
            LoweredInstructionRHS::UnaryALU { operator, arg } => write!(f, "{operator:?} {arg}"),
            LoweredInstructionRHS::BinaryALU {
                operator,
                arg1,
                arg2,
            } => write!(f, "{arg1} {operator:?} {arg2}"),
            LoweredInstructionRHS::LoadOneImmediate => write!(f, "imm"),
            LoweredInstructionRHS::StoreMemory { addr, data } => write!(f, "mem[{addr}] = {data}"),
            LoweredInstructionRHS::LoadRegister(index) => write!(f, "R[{index}]"),
            LoweredInstructionRHS::StoreRegister { index, value } => {
                write!(f, "R[{index}] = {value}")
            }
        }
    }
}

pub fn lowered_insts(
    func: &mut Function<MicrocodeConfig>,
    inst: SSAInstruction,
    input_cnt: &mut u8,
) -> impl IntoIterator<Item = LoweredInstruction> {
    match inst.rhs {
        SSAInstructionRHS::BinaryOperation {
            operator,
            arg1,
            arg2,
        } => {
            vec![LoweredInstruction {
                lhs: inst.lhs,
                rhs: LoweredInstructionRHS::BinaryALU {
                    operator: match operator {
                        BinaryOperator::Add => BinaryALUOperator::Add,
                        BinaryOperator::Mul => todo!("implement multiplication"),
                        BinaryOperator::Sub => BinaryALUOperator::Sub,
                        BinaryOperator::Div => todo!("implement division"),
                        BinaryOperator::Xor => BinaryALUOperator::Xor,
                        BinaryOperator::And => BinaryALUOperator::And,
                    },
                    arg1,
                    arg2,
                },
            }]
        }
        SSAInstructionRHS::UnaryOperation {
            operator: UnaryOperator::Not,
            arg,
        } => {
            let temp @ VirtualRegisterLValue(temp_ref) = func.new_reg();
            let temp2 @ VirtualRegisterLValue(temp2_ref) = func.new_reg();
            let temp3 @ VirtualRegisterLValue(temp3_ref) = func.new_reg();
            vec![
                LoweredInstruction {
                    lhs: temp,
                    rhs: LoweredInstructionRHS::LoadOneImmediate,
                },
                LoweredInstruction {
                    lhs: temp2,
                    rhs: LoweredInstructionRHS::UnaryALU {
                        operator: UnaryALUOperator::Dec1,
                        arg: temp_ref,
                    },
                },
                LoweredInstruction {
                    lhs: temp3,
                    rhs: LoweredInstructionRHS::UnaryALU {
                        operator: UnaryALUOperator::Dec1,
                        arg: temp2_ref,
                    },
                },
                LoweredInstruction {
                    lhs: inst.lhs,
                    rhs: LoweredInstructionRHS::BinaryALU {
                        operator: BinaryALUOperator::Xor,
                        arg1: arg,
                        arg2: temp3_ref,
                    },
                },
            ]
        }
        SSAInstructionRHS::LoadIntegerLiteral { value } => {
            let temp @ VirtualRegisterLValue(temp_ref) = func.new_reg();
            match value {
                1 => vec![LoweredInstruction {
                    lhs: inst.lhs,
                    rhs: LoweredInstructionRHS::LoadOneImmediate,
                }],
                0 => vec![
                    LoweredInstruction {
                        lhs: temp,
                        rhs: LoweredInstructionRHS::LoadOneImmediate,
                    },
                    LoweredInstruction {
                        lhs: inst.lhs,
                        rhs: LoweredInstructionRHS::UnaryALU {
                            operator: UnaryALUOperator::Dec1,
                            arg: temp_ref,
                        },
                    },
                ],
                _ => todo!("implement integer generation (aside from 0 and 1)"),
            }
        }
        SSAInstructionRHS::Move { src } => {
            println!("unexpected reg move in lowered IR");
            vec![LoweredInstruction {
                lhs: inst.lhs,
                rhs: LoweredInstructionRHS::UnaryALU {
                    operator: UnaryALUOperator::Copy,
                    arg: src,
                },
            }]
        }
        SSAInstructionRHS::ReadInput {} => {
            *input_cnt += 1;
            vec![LoweredInstruction {
                lhs: inst.lhs,
                rhs: LoweredInstructionRHS::LoadRegister(*input_cnt - 1),
            }]
        }
        SSAInstructionRHS::ReadMemory(src) => {
            vec![LoweredInstruction {
                lhs: inst.lhs,
                rhs: LoweredInstructionRHS::LoadMemory(src),
            }]
        }
    }
}

fn lower_to_microcode(_func: &SSABlock) {}
