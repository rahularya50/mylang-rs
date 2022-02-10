mod lower;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use self::lower::{lowered_insts, LoweredInstruction};
use crate::ir::{FullBlock, SSAFunction};
use crate::utils::rcequality::RcDereferencable;

enum RegisterUse {
    Memory,
    Writeback,
    Mixed,
}

pub fn gen_microops(
    mut func: SSAFunction,
) -> impl IntoIterator<Item = Rc<RefCell<FullBlock<LoweredInstruction>>>> {
    func.clear_dead_blocks();
    let mut block_lookup = HashMap::new();
    for block in func.blocks.iter() {
        block_lookup.insert(block.as_key(), Rc::new(RefCell::new(FullBlock::default())));
    }
    for block_ref in func.blocks.clone() {
        let out_block = block_lookup.remove(&block_ref.as_key()).unwrap();
        let block = block_ref.upgrade().unwrap().take();
        let mut instructions = vec![];
        for inst in block.instructions {
            instructions.extend(lowered_insts(&mut func, inst))
        }
        out_block.borrow_mut().debug_index = block.debug_index;
        out_block.borrow_mut().preds = block
            .preds
            .iter()
            .map(|pred| Rc::downgrade(&block_lookup[&pred.borrow()]).into())
            .collect();
        out_block.borrow_mut().instructions = instructions;
        block_lookup.insert(block_ref.as_key(), out_block);
    }
    block_lookup.into_values()
}
