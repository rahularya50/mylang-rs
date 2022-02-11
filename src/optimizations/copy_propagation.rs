use std::mem::take;

use itertools::Itertools;

use crate::ir::{SSAFunction, SSAInstruction, SSAInstructionRHS, VirtualRegister, WithRegisters};
use crate::utils::union_find::UnionFind;

fn make_reg_replacer(regs: &UnionFind<VirtualRegister>) -> impl Fn(&mut VirtualRegister) + '_ {
    move |reg| {
        *reg = regs.find_root(reg).map_or(*reg, |node| node.borrow().value);
    }
}

pub fn copy_propagation(func: &mut SSAFunction) {
    let mut regs = UnionFind::new();
    for block in func.blocks() {
        for inst in &block.borrow().instructions {
            if let SSAInstructionRHS::Move { src } = inst.rhs {
                regs.directed_union(src, inst.lhs.0);
            }
        }
    }

    // now, map all registers to their root
    let mapper = make_reg_replacer(&regs);

    for block in func.blocks() {
        let mut block = block.borrow_mut();
        for phi in &mut block.phis {
            phi.srcs.values_mut().for_each(&mapper);
        }
        let mut phi_moves = vec![];
        for phi in block.phis.drain_filter(|phi| phi.srcs.values().all_equal()) {
            phi_moves.push(SSAInstruction::new(
                phi.dest,
                SSAInstructionRHS::Move {
                    src: *phi
                        .srcs
                        .values()
                        .next()
                        .expect("phis must have at least one src (really, at least two!)"),
                },
            ));
        }

        for inst in &mut block.instructions {
            inst.rhs.regs_mut().for_each(&mapper);
        }

        block.exit.regs_mut().for_each(&mapper);

        phi_moves.extend(take(&mut block.instructions));
        block.instructions = phi_moves;
    }
}
