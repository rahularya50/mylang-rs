use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::Itertools;

use super::dominance::BlockDataLookup;
use super::instructions::Instruction;
use super::structs::{BlockRef, Phi, VirtualRegister, VirtualRegisterLValue, VirtualVariable};
use super::{SSABlock, SSAFunction};
use crate::utils::frame::Frame;
use crate::utils::graph::explore;
use crate::utils::rcequality::{RcDereferencable, RcEquality};

/*
see https://groups.seas.harvard.edu/courses/cs252/2011sp/slides/Lec04-SSA.pdf
also see: http://www.cs.cmu.edu/afs/cs/academic/class/15745-s12/public/lectures/L13-SSA-Concepts-1up.pdf
and http://misailo.web.engr.illinois.edu/courses/526-sp20/lec3.pdf
basic idea:
    1. compute "iterated dominance frontier" (slide 15 of harvard) for each variable,
    starting at all defining blocks, to figure out where its phi nodes should belong
    (i.e. the blocks where conflicting definitions
    need to be merged)
    2. traverse "dominator tree" (parent -> child iff parent is "lowest" dominator of child, slide 26 of CMU)
    in pre-order, and generate a new register for each variable re-definition, using a parallel frame tree
    to map variables to their latest register
    3. "backfill" the phi block sources using the frame tree
*/

pub fn defining_blocks_for_variables(
    blocks: &[BlockRef],
) -> HashMap<VirtualVariable, HashSet<RcEquality<BlockRef>>> {
    let mut out = HashMap::new();
    for block in blocks.iter() {
        for inst in &block.borrow().instructions {
            out.entry(inst.lhs)
                .or_insert_with(HashSet::new)
                .insert(block.clone().into());
        }
    }
    out
}

pub fn ssa_phis(
    func: &mut SSAFunction,
    variable_defns: &HashMap<VirtualVariable, HashSet<RcEquality<BlockRef>>>,
    frontiers: &BlockDataLookup<Vec<BlockRef>>,
) -> BlockDataLookup<HashMap<VirtualVariable, VirtualRegisterLValue>> {
    let mut out = BlockDataLookup::new();
    for (var, defns) in variable_defns.iter() {
        let mut todo = defns
            .iter()
            .map(|block| block.get_ref().clone())
            .collect_vec();
        let mut explored = HashSet::<RcEquality<BlockRef>>::new();
        while let Some(next) = todo.pop() {
            if explored.insert(next.clone().into()) {
                for frontier in frontiers.get(&next.as_key()).unwrap_or(&vec![]) {
                    out.entry(frontier.clone().into())
                        .or_insert_with(HashMap::new)
                        .insert(*var, func.new_reg());
                    todo.push(frontier.clone());
                }
            }
        }
    }
    out
}

pub fn alloc_ssa_blocks(
    func: &mut SSAFunction,
    blocks: &[BlockRef],
) -> BlockDataLookup<Rc<RefCell<SSABlock>>> {
    let mut out = HashMap::new();
    for block in blocks {
        out.insert(block.clone().into(), func.new_block());
    }
    out
}

type VirtualRegisterFrameLookup = BlockDataLookup<Frame<VirtualVariable, VirtualRegister>>;
type PhiVariableReverseLookup = BlockDataLookup<HashMap<VirtualRegister, VirtualVariable>>;

pub fn populate_ssa_blocks(
    func: &mut SSAFunction,
    start_block: BlockRef,
    mut phis: BlockDataLookup<HashMap<VirtualVariable, VirtualRegisterLValue>>,
    dominated: &BlockDataLookup<Vec<BlockRef>>,
    ssa_blocks: &BlockDataLookup<Rc<RefCell<SSABlock>>>,
) -> (VirtualRegisterFrameLookup, PhiVariableReverseLookup) {
    let mut frames = BlockDataLookup::new();
    let mut phi_vars = BlockDataLookup::new();

    explore(
        (start_block, Frame::new()),
        |(block, frame)| {
            let ssa_block = ssa_blocks
                .get(&block.as_key())
                .expect("all blocks should map to ssa blocks");
            let block_phis = phis.remove(&block.as_key());

            // override any variables from dominating nodes using phi nodes
            if let Some(block_phis) = block_phis {
                let mut block_phi_vars = HashMap::new();
                for (var, reg @ VirtualRegisterLValue(reg_ref)) in block_phis {
                    frame.assoc(var, reg_ref);
                    ssa_block.borrow_mut().phis.push(Phi {
                        srcs: HashMap::new(),
                        dest: reg,
                    });
                    block_phi_vars.insert(reg_ref, var);
                }
                phi_vars.insert(block.clone().into(), block_phi_vars);
            }

            for inst in &block.borrow().instructions {
                let rhs = inst
                    .rhs
                    .map_reg_types(frame)
                    .expect("all RHS registers should be defined in a dominating or phi block");
                let reg @ VirtualRegisterLValue(reg_ref) = func.new_reg();
                frame.assoc(inst.lhs, reg_ref);
                ssa_block
                    .borrow_mut()
                    .instructions
                    .push(Instruction::new(reg, rhs));
            }

            ssa_block.borrow_mut().exit = block
                .borrow_mut()
                .exit
                .map_reg_block_types(
                    |var| frame.lookup(var),
                    |block| ssa_blocks.get(&block.as_key()).cloned(),
                )
                .expect("all registers and blocks should already be defined/mapped");

            (
                dominated
                    .get(&block.as_key())
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|block| (block.clone(), frame.new_child()))
                    .collect_vec(),
                (),
            )
        },
        |(block, frame), _, _| {
            frames.insert(block.into(), frame);
        },
    );

    (frames, phi_vars)
}

pub fn backfill_ssa_phis(
    blocks: &[BlockRef],
    ssa_blocks: &BlockDataLookup<Rc<RefCell<SSABlock>>>,
    frames: &VirtualRegisterFrameLookup,
    phi_vars: &PhiVariableReverseLookup,
) {
    for block in blocks {
        let src_ssa_block = ssa_blocks
            .get(&block.as_key())
            .expect("all blocks must have an ssa block");
        let src_frame = frames
            .get(&block.as_key())
            .expect("all blocks must have a frame");
        for dest in block.borrow().exit.dests() {
            let dest_ssa_block = ssa_blocks
                .get(&dest.as_key())
                .expect("all blocks must have an ssa block");
            dest_ssa_block
                .borrow_mut()
                .preds
                .insert(Rc::downgrade(src_ssa_block).into());
            if let Some(dest_phi_vars) = phi_vars.get(&dest.as_key()) {
                dest_ssa_block.borrow_mut().phis.drain_filter(|phi| {
                    let Phi {
                        ref mut srcs,
                        dest: VirtualRegisterLValue(dest),
                    } = phi;
                    let var = dest_phi_vars
                        .get(dest)
                        .expect("all phi blocks must have a reverse var mapping");
                    src_frame.lookup(var).map_or(true, |src_reg| {
                        srcs.insert(Rc::downgrade(src_ssa_block).into(), src_reg);
                        false
                    })
                });
            }
        }
    }
}
