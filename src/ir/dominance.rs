use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::Itertools;

use super::structs::BlockRef;
use crate::utils::graph::explore;
use crate::utils::rcequality::{RcDereferencable, RcEquality};

pub type BlockDataLookup<T> = HashMap<RcEquality<BlockRef>, T>;

/*
Cooper, Keith D., Timothy J. Harvey, and Ken Kennedy.
"A simple, fast dominance algorithm." Software Practice & Experience 4.1-10 (2001): 1-8.
*/

pub fn sort_blocks_postorder(
    root: BlockRef,
) -> (
    Box<[BlockRef]>,
    BlockDataLookup<usize>,
    BlockDataLookup<Vec<BlockRef>>,
) {
    let mut blocks = vec![];
    let mut predecessors = HashMap::new();
    let mut visited = HashSet::<RcEquality<BlockRef>>::new();

    explore(
        root,
        |pos| {
            if visited.insert(pos.clone().into()) {
                (
                    (*pos)
                        .borrow()
                        .exit
                        .dests()
                        .into_iter()
                        .map(|dst| {
                            predecessors
                                .entry(dst.clone().into())
                                .or_insert(vec![])
                                .push(pos.clone());
                            dst.clone()
                        })
                        .collect_vec(),
                    true,
                )
            } else {
                (vec![], false)
            }
        },
        |pos, unexplored, _| {
            if unexplored {
                blocks.push(pos);
            }
        },
    );

    (
        blocks.clone().into_boxed_slice(),
        blocks
            .into_iter()
            .enumerate()
            .map(|(a, b)| (b.into(), a))
            .collect(),
        predecessors,
    )
}

// expect `blocks` to be in post-order
pub fn find_immediate_dominators(
    start_block: BlockRef,
    blocks: &[BlockRef],
    index_lookup: &BlockDataLookup<usize>,
    predecessors: &BlockDataLookup<Vec<BlockRef>>,
) -> BlockDataLookup<BlockRef> {
    let mut dominators = BlockDataLookup::new();
    dominators.insert(start_block.clone().into(), start_block);
    let mut changed = true;
    while changed {
        changed = false;
        for node in blocks.iter().rev().skip(1) {
            let node_key = node.clone().into();

            let node_preds = predecessors
                .get(&node_key)
                .expect("all blocks but the root should have a predecessor");

            let idom = node_preds
                .iter()
                .cloned()
                .filter(|x| dominators.contains_key(&x.as_key()))
                .reduce(|a, b| intersect(a, b, index_lookup, &dominators))
                .expect("current node should have a predecessor with dominance computated");

            if let Some(old) = dominators.get(&node_key) {
                if Rc::ptr_eq(old, &idom) {
                    continue;
                }
            }

            dominators.insert(node_key, idom);
            changed = true;
        }
    }
    dominators
}

fn intersect(
    mut a: BlockRef,
    mut b: BlockRef,
    index_lookup: &BlockDataLookup<usize>,
    dominators: &BlockDataLookup<BlockRef>,
) -> BlockRef {
    while !Rc::ptr_eq(&a, &b) {
        let dominator_error = "all blocks should be in dominators while performing intersection";
        let index_error = "all blocks should be in index lookup";

        while index_lookup.get(&a.as_key()).expect(index_error)
            < index_lookup.get(&b.as_key()).expect(index_error)
        {
            a = dominators.get(&a.as_key()).expect(dominator_error).clone();
        }
        while index_lookup.get(&b.as_key()).expect(index_error)
            < index_lookup.get(&a.as_key()).expect(index_error)
        {
            b = dominators.get(&b.as_key()).expect(dominator_error).clone();
        }
    }
    a
}

pub fn find_immediately_dominated(
    blocks: &[BlockRef],
    dominators: &BlockDataLookup<BlockRef>,
) -> BlockDataLookup<Vec<BlockRef>> {
    let mut dominated = BlockDataLookup::new();
    for block in blocks {
        let dom = dominators
            .get(&block.as_key())
            .expect("block must have dominator");
        if Rc::ptr_eq(block, dom) {
            // it's the root node, so it's a special case
            continue;
        }
        dominated
            .entry(dom.clone().into())
            .or_insert(vec![])
            .push(block.clone());
    }
    dominated
}

pub fn dominance_frontiers(
    blocks: &[BlockRef],
    predecessors: &BlockDataLookup<Vec<BlockRef>>,
    dominators: &BlockDataLookup<BlockRef>,
) -> BlockDataLookup<Vec<BlockRef>> {
    let mut frontiers = BlockDataLookup::new();
    for block in blocks {
        if let Some(preds) = predecessors.get(&block.as_key()) {
            if preds.len() > 1 {
                for pred in preds.clone() {
                    let mut pos = pred;
                    let dom = dominators
                        .get(&block.as_key())
                        .expect("block must have dominator");
                    while pos.as_key() != dom.as_key() {
                        frontiers
                            .entry(pos.clone().into())
                            .or_insert(vec![])
                            .push(block.clone());
                        pos = dominators
                            .get(&pos.as_key())
                            .expect("block must have dominator")
                            .clone();
                    }
                }
            }
        }
    }
    frontiers
}
