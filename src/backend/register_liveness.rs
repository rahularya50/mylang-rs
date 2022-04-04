use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::mem::discriminant;
use std::rc::{Rc, Weak};

use itertools::Itertools;

use super::microcode::Block;
use crate::ir::{Instruction, VirtualRegister, VirtualRegisterLValue, WithRegisters};
use crate::utils::rcequality::{RcDereferencable, RcEquality};

#[derive(Debug, PartialEq, Eq)]
pub enum DefiningPosition {
    Before,
    Phi(usize),
    Instruction(usize),
}

#[derive(Debug)]
pub struct PhiConsumer<BType> {
    pub index: usize,
    pub src: RcEquality<Weak<RefCell<BType>>>,
}

#[derive(Debug)]
pub enum ConsumingPosition<BType> {
    Phi(PhiConsumer<BType>),
    Instruction(usize),
    Jump,
    After,
}

fn pos_cmp<BType>(left: &DefiningPosition, right: &ConsumingPosition<BType>) -> Option<Ordering> {
    match left {
        DefiningPosition::Before => Some(Ordering::Less),
        DefiningPosition::Phi(defining_phi) => match right {
            ConsumingPosition::Phi(PhiConsumer { index, .. }) => defining_phi.partial_cmp(index),
            _ => Some(Ordering::Less),
        },
        DefiningPosition::Instruction(i) => match right {
            ConsumingPosition::Phi(_) => Some(Ordering::Greater),
            ConsumingPosition::Instruction(j) => i.partial_cmp(j),
            _ => Some(Ordering::Less),
        },
    }
}

impl<BType> PartialEq<ConsumingPosition<BType>> for DefiningPosition {
    fn eq(&self, other: &ConsumingPosition<BType>) -> bool {
        pos_cmp(self, other) == Some(Ordering::Equal)
    }
}

impl<BType> PartialOrd<ConsumingPosition<BType>> for DefiningPosition {
    fn partial_cmp(&self, other: &ConsumingPosition<BType>) -> Option<Ordering> {
        pos_cmp(self, other)
    }
}

#[derive(Debug)]
pub struct RegisterLiveness<BType> {
    pub since_index: DefiningPosition,
    pub until_index: ConsumingPosition<BType>,
}

pub fn find_liveness<RValue>(
    blocks: &Vec<Rc<RefCell<Block<RValue>>>>,
    reg: VirtualRegister,
) -> HashMap<RcEquality<Rc<RefCell<Block<RValue>>>>, RegisterLiveness<Block<RValue>>>
where
    Instruction<VirtualRegisterLValue, RValue>: WithRegisters<VirtualRegister>,
{
    let mut out: HashMap<RcEquality<_>, _> = HashMap::new();
    let mut todo = vec![];
    'blocks: for block in blocks {
        if block.borrow().exit.regs().contains(&reg) {
            todo.push((block.clone(), ConsumingPosition::<Block<RValue>>::Jump));
            continue 'blocks;
        }

        for (index, inst) in block.borrow().instructions.iter().enumerate().rev() {
            if inst.regs().contains(&reg) {
                todo.push((block.clone(), ConsumingPosition::Instruction(index)));
                continue 'blocks;
            }
        }

        let mut found = false;
        for (index, phi) in block.borrow().phis.iter().enumerate().rev() {
            if let Some((pred_block, _)) = phi.srcs.iter().find(|(_block, src)| **src == reg) {
                if !found {
                    found = true;
                    let position = {
                        ConsumingPosition::Phi(PhiConsumer {
                            index,
                            src: pred_block.0.clone().into(),
                        })
                    };
                    out.insert(
                        block.clone().into(),
                        RegisterLiveness {
                            since_index: DefiningPosition::Before,
                            until_index: position,
                        },
                    );
                }
                todo.push((
                    pred_block
                        .get_ref()
                        .upgrade()
                        .expect("phis should not point to dropped blocks")
                        .clone(),
                    ConsumingPosition::After,
                ));
            }
        }
    }

    'todo: while let Some((block, latest_use)) = todo.pop() {
        let liveness = out.get(&block.as_key());
        if let Some(liveness) = liveness {
            if matches!(liveness.until_index, ConsumingPosition::After) {
                // entire block is already traversed
                continue;
            }
        }

        let entry = out
            .entry(block.clone().into())
            // invariant: only one non-AFTER latest_use will be in todo per block, so we can use this to simplfify the max
            .and_modify(|e| e.until_index = ConsumingPosition::After)
            .or_insert(RegisterLiveness {
                since_index: DefiningPosition::Before,
                until_index: latest_use,
            });

        // check to see if consumer is the definer
        for (i, phi) in block.borrow().phis.iter().enumerate() {
            if phi.dest.0 == reg {
                entry.since_index = DefiningPosition::Phi(i);
                continue 'todo;
            }
        }
        for (i, inst) in block.borrow().instructions.iter().enumerate() {
            if inst.lhs.0 == reg {
                entry.since_index = DefiningPosition::Instruction(i);
                continue 'todo;
            }
        }
        todo.extend(
            block
                .borrow()
                .preds()
                .map(|pred| (pred, ConsumingPosition::After)),
        )
    }

    out
}
