use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::ir::{SSABlock, SSAFunction, SSAJumpInstruction};
use crate::utils::rcequality::RcEquality;

pub fn remove_empty_blocks(func: &mut SSAFunction) {
    let mut visited = HashSet::<RcEquality<RefCell<SSABlock>>>::new();
    while let Some(block_to_remove) = func.blocks().find(|block| {
        !visited.contains(&Rc::as_ptr(block))
            && block.borrow().instructions.is_empty()
            && block.borrow().phis.is_empty()
            && matches!(
                block.borrow().exit,
                SSAJumpInstruction::UnconditionalJump { .. }
            )
    }) {
        visited.insert(block_to_remove.clone().into());
        if let SSAJumpInstruction::UnconditionalJump { dest: _ } = &block_to_remove.borrow().exit {
        } else {
            panic!("unexpected")
        }
        // func.blocks()
    }
    func.clear_dead_blocks();
}
