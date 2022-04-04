use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::ir::{SSAFunction, SSAInstruction, SSAJumpInstruction, SSAPhi, WithRegisters};

enum RegisterUsage<'a> {
    Assignment(&'a SSAInstruction),
    Jump(&'a SSAJumpInstruction),
    Phi(&'a SSAPhi),
}

enum RegisterDefinition<'a> {
    Assignment(&'a SSAInstruction),
    Phi(&'a SSAPhi),
}

pub fn remove_dead_statements(func: &mut SSAFunction) {
    let mut initially_live_registers = HashSet::new();
    let mut register_definers = HashMap::new();
    let mut register_users = HashMap::<_, Vec<_>>::new();
    let blocks = func.blocks().collect_vec();
    let blocks = blocks.iter().map(|block| block.borrow()).collect_vec();

    for block in &blocks {
        for phi in &block.phis {
            register_definers.insert(phi.dest.0, RegisterDefinition::Phi(phi));
            for reg in phi.srcs.values() {
                register_users
                    .entry(*reg)
                    .or_default()
                    .push(RegisterUsage::Phi(phi));
            }
        }
        for inst in &block.instructions {
            register_definers.insert(inst.lhs.0, RegisterDefinition::Assignment(inst));
            for reg in inst.rhs.regs() {
                register_users
                    .entry(*reg)
                    .or_default()
                    .push(RegisterUsage::Assignment(inst));
            }
        }
        for reg in block.exit.regs() {
            register_users
                .entry(*reg)
                .or_default()
                .push(RegisterUsage::Jump(&block.exit));
        }
        for reg in block.exit.regs() {
            initially_live_registers.insert(reg);
        }
    }

    let mut registers_to_process = initially_live_registers.into_iter().copied().collect_vec();
    let mut processed_registers = HashSet::new();

    while let Some(next_reg) = registers_to_process.pop() {
        if processed_registers.insert(next_reg) {
            let defn = register_definers.get(&next_reg).unwrap();
            match defn {
                RegisterDefinition::Assignment(inst) => {
                    registers_to_process.extend(inst.rhs.regs().copied());
                }
                RegisterDefinition::Phi(phi) => {
                    registers_to_process.extend(phi.srcs.values().copied());
                }
            }
        }
    }

    drop(blocks); // so we can safely mutate!

    for block in func.blocks() {
        block
            .borrow_mut()
            .phis
            .retain(|phi| processed_registers.contains(&phi.dest.0));
        block
            .borrow_mut()
            .instructions
            .retain(|inst| processed_registers.contains(&inst.lhs.0));
    }
}
