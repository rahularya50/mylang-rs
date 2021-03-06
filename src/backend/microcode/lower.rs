use super::instructions::LoweredInstructionRHS;
use crate::backend::lower_func::lower;
use crate::backend::microcode::instructions::lowered_insts;
use crate::ir::{
    CfgConfig, FullBlock, Function, JumpInstruction, SSAFunction, SSAJumpInstruction,
    VirtualRegister, VirtualRegisterLValue,
};
use crate::utils::rcequality::RcDereferencable;

enum RegisterUse {
    Memory,
    Writeback,
    Mixed,
}

#[derive(Debug)]
pub struct MicrocodeConfig;

impl CfgConfig for MicrocodeConfig {
    type LValue = VirtualRegisterLValue;
    type RValue = VirtualRegister;
    type RHSType = LoweredInstructionRHS<VirtualRegister>;
    type BlockType = FullBlock<Self>;
}

pub fn lower_func(func: SSAFunction) -> Function<MicrocodeConfig> {
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
}
