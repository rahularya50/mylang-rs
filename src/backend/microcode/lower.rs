use std::fmt::{self, Display, Formatter};

use crate::ir::{
    SSABlock, SSAFunction, SSAInstruction, SSAInstructionRHS, VirtualRegister,
    VirtualRegisterLValue,
};
use crate::semantics::{BinaryOperator, UnaryOperator};

pub struct LoweredInstruction {
    pub lhs: VirtualRegisterLValue,
    pub rhs: LoweredInstructionRHS,
}

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

pub enum LoweredInstructionRHS {
    UnaryALU {
        operator: UnaryALUOperator,
        arg: VirtualRegister,
    },
    BinaryALU {
        operator: BinaryALUOperator,
        arg1: VirtualRegister,
        arg2: VirtualRegister,
    },
    LoadOneImmediate,
    LoadMemory(VirtualRegister),
    StoreMemory {
        addr: VirtualRegister,
        data: VirtualRegister,
    },
    LoadRegister(u8),
    StoreRegister {
        index: u8,
        value: VirtualRegister,
    },
}

impl Display for LoweredInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} = ", self.lhs)?;
        match &self.rhs {
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
    func: &mut SSAFunction,
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
        SSAInstructionRHS::LoadIntegerLiteral { value: _ } => {
            todo!("implement integer generation")
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
