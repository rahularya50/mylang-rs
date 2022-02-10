use self::block_merging::remove_empty_blocks;
use self::copy_propagation::copy_propagation;
use self::dead_code_elimination::remove_dead_statements;
use self::simplify_jumps::simplify_jumps;
use crate::ir::SSAFunction;
use crate::optimizations::constant_folding::constant_folding;
use crate::semantics::Program;

mod block_merging;
mod constant_folding;
mod copy_propagation;
mod dead_code_elimination;
mod simplify_jumps;

pub fn optimize(program: &mut Program<SSAFunction>, fold_constants: bool) {
    // inter-procedural optimizations
    for func in program.funcs.values_mut() {
        // note: this MUST run first to remove optimistic but invalid phis
        remove_dead_statements(func);

        for _ in 0..5 {
            remove_dead_statements(func);
            remove_empty_blocks(func);
            simplify_jumps(func);
            if fold_constants {
                constant_folding(func);
            }
            copy_propagation(func);
            func.clear_dead_blocks();
        }
    }
}
