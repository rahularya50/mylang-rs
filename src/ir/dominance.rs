use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::structs::BlockRef;
use crate::utils::RcEquality;

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

    fn explore(
        pos: BlockRef,
        blocks: &mut Vec<BlockRef>,
        predecessors: &mut BlockDataLookup<Vec<BlockRef>>,
        visited: &mut HashSet<RcEquality<BlockRef>>,
    ) {
        if visited.insert(pos.clone().into()) {
            for dst in (*pos).borrow().exit.dests() {
                predecessors
                    .entry(dst.clone().into())
                    .or_insert(vec![])
                    .push(pos.clone());
                explore(dst, blocks, predecessors, visited);
            }
            blocks.push(pos);
        }
    }

    explore(root, &mut blocks, &mut predecessors, &mut HashSet::new());
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
    let mut dominators = HashMap::new();
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
                .filter(|x| dominators.contains_key(&x.clone().into()))
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

        while index_lookup.get(&a.clone().into()).expect(index_error)
            < index_lookup.get(&b.clone().into()).expect(index_error)
        {
            a = dominators
                .get(&a.clone().into())
                .expect(dominator_error)
                .clone();
        }
        while index_lookup.get(&b.clone().into()).expect(index_error)
            < index_lookup.get(&a.clone().into()).expect(index_error)
        {
            b = dominators
                .get(&b.clone().into())
                .expect(dominator_error)
                .clone();
        }
    }
    a
}

pub fn find_immediately_dominated(
    blocks: &[BlockRef],
    dominators: &BlockDataLookup<BlockRef>,
) -> BlockDataLookup<Vec<BlockRef>> {
    let mut dominated = HashMap::new();
    for block in blocks {
        let dom = dominators
            .get(&block.clone().into())
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
    let mut frontiers = HashMap::new();
    for block in blocks {
        if let Some(preds) = predecessors.get(&block.clone().into()) {
            if preds.len() > 1 {
                for pred in preds.clone() {
                    let mut pos = pred;
                    let dom = dominators
                        .get(&block.clone().into())
                        .expect("block must have dominator");
                    while !Rc::ptr_eq(&pos, dom) {
                        frontiers
                            .entry(pos.clone().into())
                            .or_insert(vec![])
                            .push(block.clone());
                        pos = dominators
                            .get(&pos.into())
                            .expect("block must have dominator")
                            .clone();
                    }
                }
            }
        }
    }
    frontiers
}