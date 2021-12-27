use self::block_merging::remove_empty_blocks;
use crate::ir::SSAFunction;

mod block_merging;

pub fn optimize(func: &mut SSAFunction) {
    remove_empty_blocks(func);
}
