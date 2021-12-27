use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::Itertools;

use super::dominance::BlockDataLookup;
use super::instructions::Instruction;
use super::structs::{
    BlockRef, Function, Phi, SSABlock, VirtualRegister, VirtualRegisterLValue, VirtualVariable,
};
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

pub fn ssa_phis<T>(
    func: &mut Function<VirtualRegisterLValue, T>,
    variable_defns: &HashMap<VirtualVariable, HashSet<RcEquality<BlockRef>>>,
    frontiers: &BlockDataLookup<Vec<BlockRef>>,
) -> BlockDataLookup<HashMap<VirtualVariable, VirtualRegisterLValue>> {
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
                        .insert(*var, func.new_reg());
                    todo.push(frontier.clone());
                }
            }
        }
    }
    out
}

pub fn alloc_ssa_blocks<T>(
    func: &mut Function<T, SSABlock>,
    blocks: &[BlockRef],
) -> BlockDataLookup<Rc<RefCell<SSABlock>>> {
    let mut out = HashMap::new();
    for block in blocks {
        out.insert(block.clone().into(), func.new_block());
    }
    out
}

pub fn populate_ssa_blocks<T>(
    func: &mut Function<VirtualRegisterLValue, T>,
    start_block: BlockRef,
    mut phis: BlockDataLookup<HashMap<VirtualVariable, VirtualRegisterLValue>>,
    dominated: &BlockDataLookup<Vec<BlockRef>>,
    ssa_blocks: &BlockDataLookup<Rc<RefCell<SSABlock>>>,
) -> (
    BlockDataLookup<Frame<VirtualVariable, VirtualRegister>>,
    BlockDataLookup<HashMap<VirtualRegister, VirtualVariable>>,
) {
    let mut frames = BlockDataLookup::new();
    let mut phi_vars = BlockDataLookup::new();

    fn explore<T>(
        func: &mut Function<VirtualRegisterLValue, T>,
        start_block: BlockRef,
        frames: &mut BlockDataLookup<Frame<VirtualVariable, VirtualRegister>>,
        phis: &mut BlockDataLookup<HashMap<VirtualVariable, VirtualRegisterLValue>>,
        phi_vars: &mut BlockDataLookup<HashMap<VirtualRegister, VirtualVariable>>,
        dominated: &BlockDataLookup<Vec<BlockRef>>,
        ssa_blocks: &BlockDataLookup<Rc<RefCell<SSABlock>>>,
        mut frame: Frame<VirtualVariable, VirtualRegister>,
    ) {
        let block = ssa_blocks
            .get(&start_block.clone().into())
            .expect("all blocks should map to ssa blocks");
        let block_phis = phis.remove(&start_block.clone().into());

        // override any variables from dominating nodes using phi nodes
        if let Some(block_phis) = block_phis {
            let mut block_phi_vars = HashMap::new();
            for (var, reg @ VirtualRegisterLValue(reg_ref)) in block_phis.into_iter() {
                // do not allow variables to be speculatively defined
                // they must have a definition from a dominating node, even if it is always overridden
                // this is mostly a safety check, we can relax it later without violating correctness
                // if we add a pass to prune invalid phis
                if frame.lookup(&var).is_some() {
                    frame.assoc(var, reg_ref);
                    block.borrow_mut().phis.push(Phi {
                        srcs: vec![],
                        dest: reg,
                    });
                    block_phi_vars.insert(reg_ref, var);
                }
            }
            phi_vars.insert(start_block.clone().into(), block_phi_vars);
        }

        for inst in start_block.borrow().instructions.iter() {
            let rhs = inst
                .rhs
                .replace_regs(&frame)
                .expect("all RHS registers should be defined in a dominating or phi block");
            let reg @ VirtualRegisterLValue(reg_ref) = func.new_reg();
            frame.assoc(inst.lhs, reg_ref);
            block
                .borrow_mut()
                .instructions
                .push(Instruction::new(reg, rhs));
        }

        block.borrow_mut().exit = start_block
            .borrow_mut()
            .exit
            .replace(&frame, ssa_blocks)
            .expect("all registers and blocks should already be defined/mapped");

        for dominated_block in dominated
            .get(&start_block.clone().into())
            .unwrap_or(&vec![])
        {
            explore(
                func,
                dominated_block.clone(),
                frames,
                phis,
                phi_vars,
                dominated,
                ssa_blocks,
                frame.new_child(),
            );
        }

        frames.insert(start_block.into(), frame);
    }
    explore(
        func,
        start_block,
        &mut frames,
        &mut phis,
        &mut phi_vars,
        dominated,
        ssa_blocks,
        Frame::new(),
    );

    (frames, phi_vars)
}

pub fn backfill_ssa_phis(
    blocks: &[BlockRef],
    ssa_blocks: &BlockDataLookup<Rc<RefCell<SSABlock>>>,
    phi_vars: &BlockDataLookup<HashMap<VirtualRegister, VirtualVariable>>,
    frames: &BlockDataLookup<Frame<VirtualVariable, VirtualRegister>>,
) {
    for block in blocks {
        let src_ssa_block = ssa_blocks
            .get(&block.clone().into())
            .expect("all blocks must have an ssa block");
        let src_frame = frames
            .get(&block.clone().into())
            .expect("all blocks must have a frame");
        for dest in block.borrow().exit.dests() {
            let dest_ssa_block = ssa_blocks
                .get(&dest.clone().into())
                .expect("all blocks must have an ssa block");
            if let Some(dest_phi_vars) = phi_vars.get(&dest.clone().into()) {
                for Phi {
                    ref mut srcs,
                    dest: VirtualRegisterLValue(dest),
                } in &mut dest_ssa_block.borrow_mut().phis
                {
                    let var = dest_phi_vars
                        .get(&dest)
                        .expect("all phi blocks must have a reverse var mapping");
                    let src_reg = src_frame
                        .lookup(var)
                        .expect("phi srcs must be defined in each possible input");
                    srcs.push((src_reg, Rc::downgrade(src_ssa_block)));
                }
            }
        }
    }
}
