use crate::ir::{SSAFunction, SSAJumpInstruction};
use crate::utils::rcequality::RcEqualityKey;

pub fn simplify_jumps(func: &mut SSAFunction) {
    for block in func.blocks() {
        let mut block = block.borrow_mut();
        if let SSAJumpInstruction::BranchIfElseZero {
            ref conseq,
            ref alt,
            ..
        } = block.exit
        {
            if conseq.as_key() == alt.as_key() {
                block.exit = SSAJumpInstruction::UnconditionalJump {
                    dest: conseq.clone(),
                }
            }
        }
    }
}
