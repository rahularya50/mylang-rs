use self::block_merging::remove_empty_blocks;
use self::dead_code_elimination::remove_dead_statements;
use self::simplify_jumps::simplify_jumps;
use crate::ir::SSAFunction;

mod block_merging;
mod dead_code_elimination;
mod simplify_jumps;

pub fn optimize(func: &mut SSAFunction) {
    for _ in 0..5 {
        remove_dead_statements(func);
        remove_empty_blocks(func);
        simplify_jumps(func);
    }
}
