use self::block_merging::remove_empty_blocks;
use self::dead_code_elimination::remove_dead_statements;
use crate::ir::SSAFunction;

mod block_merging;
mod dead_code_elimination;

pub fn optimize(func: &mut SSAFunction) {
    remove_empty_blocks(func);
    remove_dead_statements(func);
}
