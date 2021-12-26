use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use super::dominance::BlockDataLookup;
use super::structs::{BlockRef, Function, Phi, VirtualRegister, VirtualVariable};
use crate::utils::{Frame, RcEquality};

pub fn defining_blocks_for_variables(
    blocks: &[BlockRef],
) -> HashMap<VirtualVariable, HashSet<RcEquality<BlockRef>>> {
    let mut out = HashMap::new();
    for block in blocks.iter() {
        for inst in block.borrow().instructions.iter() {
            out.entry(inst.lhs)
                .or_insert_with(HashSet::new)
                .insert(block.clone().into());
        }
    }
    out
}

pub fn ssa_phis(
    func: &mut Function,
    variable_defns: &HashMap<VirtualVariable, HashSet<RcEquality<BlockRef>>>,
    frontiers: &BlockDataLookup<Vec<BlockRef>>,
) -> BlockDataLookup<HashMap<VirtualVariable, Phi>> {
    let mut out = HashMap::new();
    for (var, defns) in variable_defns.iter() {
        let mut todo = defns
            .iter()
            .map(|RcEquality(block)| block.clone())
            .collect_vec();
        let mut explored = HashSet::<RcEquality<BlockRef>>::new();
        while let Some(next) = todo.pop() {
            if explored.insert(next.clone().into()) {
                for frontier in frontiers.get(&next.clone().into()).unwrap_or(&vec![]) {
                    out.entry(frontier.clone().into())
                        .or_insert_with(HashMap::new)
                        .insert(
                            *var,
                            Phi {
                                srcs: vec![],
                                dest: func.new_reg(),
                            },
                        );
                    todo.push(frontier.clone());
                }
            }
        }
    }
    out
}

pub fn gen_ssa_blocks(
    start_block: BlockRef,
    _dominated: &BlockDataLookup<Vec<BlockRef>>,
    frame: Frame<VirtualVariable, VirtualRegister>,
) {
    for inst in start_block.borrow().instructions.iter() {
        inst.rhs.replace_regs(&frame);
    }
}
