
use std::collections::{HashMap, HashSet};


use itertools::Itertools;

use crate::ir::{Phi, SSAFunction, SSAInstruction, SSAJumpInstruction};


enum RegisterUsage<'a> {
    Assignment(&'a SSAInstruction),
    Jump(&'a SSAJumpInstruction),
    Phi(&'a Phi),
}
enum RegisterDefinition<'a> {
    Assignment(&'a SSAInstruction),
    Phi(&'a Phi),
}

pub fn remove_dead_statements(func: &mut SSAFunction) {
    let mut live_registers = HashSet::new();
    let mut register_definers = HashMap::new();
    let mut register_users = HashMap::<_, Vec<_>>::new();
    let blocks = func.blocks().collect_vec();
    let blocks = blocks.iter().map(|block| block.borrow()).collect_vec();

    for block in &blocks {
        for phi in &block.phis {
            register_definers.insert(&phi.dest, RegisterDefinition::Phi(phi));
            for reg in phi.srcs.values() {
                register_users
                    .entry(*reg)
                    .or_default()
                    .push(RegisterUsage::Phi(phi));
            }
        }
        for inst in &block.instructions {
            register_definers.insert(&inst.lhs, RegisterDefinition::Assignment(inst));
            for reg in inst.rhs.regs() {
                register_users
                    .entry(*reg)
                    .or_default()
                    .push(RegisterUsage::Assignment(inst));
            }
        }
        for reg in block.exit.srcs() {
            register_users
                .entry(*reg)
                .or_default()
                .push(RegisterUsage::Jump(&block.exit));
        }
        if let SSAJumpInstruction::Ret(Some(reg)) = block.exit {
            live_registers.insert(reg);
        }
    }

    let mut registers_to_process = live_registers.iter().copied().collect_vec();

    while let Some(next_reg) = registers_to_process.pop() {
        for _user in register_users.get(&next_reg).unwrap_or(&vec![]) {}
    }
}
