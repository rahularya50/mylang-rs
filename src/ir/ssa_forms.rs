use std::fmt::Debug;

use super::instructions::InstructionRHS;
use super::structs::{Block, BlockWithDebugIndex, VirtualVariable};
use super::{FullBlock, RegisterLValue, VirtualRegister, VirtualRegisterLValue, WithRegisters, Instruction};

pub trait CfgConfig: Debug {
    type LValue: RegisterLValue<RValue = Self::RValue> + Debug;
    type RValue: Eq + Copy + Debug;
    type RHSType: WithRegisters<Self::RValue> + Debug;
    type BlockType: BlockWithDebugIndex;
}

#[derive(Debug)]
pub struct InitialCfg;

impl CfgConfig for InitialCfg {
    type LValue = VirtualVariable;
    type RValue = VirtualVariable;
    type RHSType = InstructionRHS<VirtualVariable>;
    type BlockType = Block;
}

#[derive(Debug)]
pub struct SSAConfig;

impl CfgConfig for SSAConfig {
    type LValue = VirtualRegisterLValue;
    type RValue = VirtualRegister;
    type RHSType = InstructionRHS<VirtualRegister>;
    type BlockType = FullBlock<Self>;
}

