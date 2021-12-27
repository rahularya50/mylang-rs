use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::ir::{SSABlock, SSAFunction, SSAJumpInstruction};
use crate::utils::rcequality::{RcEquality, RcEqualityKey};

pub fn remove_empty_blocks(func: &mut SSAFunction) {
    let mut visited = HashSet::<RcEquality<Rc<RefCell<SSABlock>>>>::new();
    while let Some(block_to_remove) = func.blocks().find(|block| {
        !visited.contains(&block.as_key())
            && block.borrow().instructions.is_empty()
            && block.borrow().phis.is_empty()
            && matches!(
                block.borrow().exit,
                SSAJumpInstruction::UnconditionalJump { .. }
            )
    }) {
        visited.insert(block_to_remove.clone().into());
        if let SSAJumpInstruction::UnconditionalJump { dest: _ } = &block_to_remove.borrow().exit {
            // we will attempt to delete this block
            // all predecessor nodes will instead jump directly to the dest
            // we have no phi nodes - however, our dest may have phis
            // at each step, we will "redirect" a predecessor straight to the dest
            // but we will skip the redirection if this results in a phi conflict in the dest
            // for pred in block_to_remove.borrow().preds() {
            //     for phi in dest.borrow().phis {
            //         //
            //     }
            // }
        } else {
            panic!("unexpected")
        }
        // func.blocks()
    }
    func.clear_dead_blocks();
}
