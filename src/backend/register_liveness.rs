use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::ir::{FullBlock, Instruction, VirtualRegister, VirtualRegisterLValue, WithRegisters};
use crate::utils::rcequality::{RcDereferencable, RcEquality};

#[derive(Copy, Clone, Debug)]
pub enum BlockPosition {
    Phi(usize),
    Instruction(usize),
    Jump,
}

#[derive(Debug, Default)]
pub struct RegisterLiveness {
    pub since_index: Option<BlockPosition>,
    pub until_index: Option<BlockPosition>,
}

type Block<RValue> = FullBlock<Instruction<VirtualRegisterLValue, RValue>>;

pub fn find_liveness<RValue>(
    blocks: &Vec<Rc<RefCell<Block<RValue>>>>,
    reg: VirtualRegister,
) -> HashMap<RcEquality<Rc<RefCell<Block<RValue>>>>, RegisterLiveness>
where
    Instruction<VirtualRegisterLValue, RValue>: WithRegisters<VirtualRegister>,
{
    let mut out: HashMap<RcEquality<_>, _> = HashMap::new();
    let mut todo = vec![];
    for block in blocks {
        let mut latest_use = None;

        if block.borrow().exit.regs().contains(&reg) {
            latest_use = Some(BlockPosition::Jump);
        }

        if latest_use.is_none() {
            for (index, inst) in block.borrow().instructions.iter().enumerate().rev() {
                if inst.regs().contains(&reg) {
                    latest_use = Some(BlockPosition::Instruction(index));
                    todo.push((block.clone(), latest_use));
                    break;
                }
            }
        }

        if latest_use.is_none() {
            for (index, phi) in block.borrow().phis.iter().enumerate().rev() {
                if let Some((pred_block, _)) = phi.srcs.iter().find(|(block, src)| **src == reg) {
                    if latest_use.is_none() {
                        latest_use = Some(BlockPosition::Phi(index));
                        out.insert(
                            block.clone().into(),
                            RegisterLiveness {
                                since_index: None,
                                until_index: latest_use,
                            },
                        );
                    }
                    todo.push((
                        pred_block
                            .get_ref()
                            .upgrade()
                            .expect("phis should not point to dropped blocks")
                            .clone(),
                        None,
                    ));
                    break;
                }
            }
        }

        if latest_use.is_some() {
            out.insert(
                block.clone().into(),
                RegisterLiveness {
                    since_index: None,
                    until_index: latest_use,
                },
            );
        }
    }

    while let Some((block, latest_use)) = todo.pop() {
        let liveness = out.get(&block.as_key());
        if let Some(liveness) = liveness {
            if liveness.until_index.is_none() {
                // entire block is already traversed
                continue;
            }
        }
        let entry = out.entry(block.clone().into()).or_default();
        entry.until_index = latest_use;
        // check to see if consumer is the definer
        let mut found = false;
        for (i, phi) in block.borrow().phis.iter().enumerate() {
            if phi.dest.0 == reg {
                entry.since_index = Some(BlockPosition::Phi(i));
                found = true;
                break;
            }
        }
        for (i, inst) in block.borrow().instructions.iter().enumerate() {
            if inst.lhs.0 == reg {
                entry.since_index = Some(BlockPosition::Instruction(i));
                found = true;
                break;
            }
        }
        if !found {
            todo.extend(block.borrow().preds().map(|pred| (pred, None)))
        }
    }

    out
}
