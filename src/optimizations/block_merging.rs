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
        if let SSAJumpInstruction::UnconditionalJump { dest } = &block_to_remove.borrow().exit {
            println!(
                "Attempting to delete block {}",
                block_to_remove.borrow().debug_index
            );
            // we will attempt to delete this block
            // all predecessor nodes will instead jump directly to the dest
            // we have no phi nodes - however, our dest may have phis
            // at each step, we will "redirect" a predecessor straight to the dest
            // but we will skip the redirection if this results in a phi conflict in the dest
            for pred in block_to_remove.borrow().preds() {
                let risky_phi = dest.borrow().phis.iter().any(|phi| {
                    let block_reg = phi
                        .srcs
                        .get(&block_to_remove.as_key())
                        .expect("phis should include all preds");
                    phi.srcs.get(&pred.as_key()).map(|reg| reg != block_reg) == Some(false)
                });

                if !risky_phi {
                    println!(
                        "Removing edge from block {}->{} and {}->{}",
                        pred.borrow().debug_index,
                        block_to_remove.borrow().debug_index,
                        block_to_remove.borrow().debug_index,
                        dest.borrow().debug_index,
                    ); // redirect pred straight to dest
                    for old_dest in pred.borrow_mut().exit.dests_mut() {
                        if old_dest.as_key() == block_to_remove.as_key() {
                            *old_dest = dest.clone();
                        }
                    }
                    for phi in &mut dest.borrow_mut().phis {
                        let block_reg = *phi
                            .srcs
                            .get(&block_to_remove.as_key())
                            .expect("phis should include all preds");
                        phi.srcs.insert(Rc::downgrade(&pred).into(), block_reg);
                        phi.srcs.remove(&block_to_remove.as_key());
                    }
                    dest.borrow_mut().preds.remove(&block_to_remove.as_key());
                    dest.borrow_mut().preds.insert(Rc::downgrade(&pred).into());
                }
            }
        } else {
            panic!("unexpected")
        }
        // func.blocks()
    }
    func.clear_dead_blocks();
}
