use self::block_merging::remove_empty_blocks;
use self::constant_propagation::constant_propagation;
use self::dead_code_elimination::remove_dead_statements;
use self::simplify_jumps::simplify_jumps;
use crate::ir::SSAFunction;

mod block_merging;
mod constant_propagation;
mod dead_code_elimination;
mod simplify_jumps;

pub fn optimize(func: &mut SSAFunction) {
    // note: this MUST run first to remove optimistic but invalid phis
    remove_dead_statements(func);

    for _ in 0..5 {
        remove_dead_statements(func);
        remove_empty_blocks(func);
        simplify_jumps(func);
        constant_propagation(func);
    }
}
