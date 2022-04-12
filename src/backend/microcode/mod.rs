use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::iter::empty;

use itertools::Itertools;

use self::instructions::LoweredInstructionRHS;
use self::lower::lower_func;
use super::lower_func::lower;
use super::register_coloring::{
    build_register_graph, color_registers, PhysicalRegister, RegisterAllocation,
};
use super::register_liveness::find_liveness;
use crate::ir::{CfgConfig, FullBlock, Instruction, SSAFunction};
use crate::utils::rcequality::RcDereferencable;

mod instructions;
mod lower;

#[derive(Debug)]
pub struct AllocatedMicrocodeConfig;

impl CfgConfig for AllocatedMicrocodeConfig {
    type LValue = PhysicalRegister;
    type RValue = PhysicalRegister;
    type RHSType = LoweredInstructionRHS<PhysicalRegister>;
    type BlockType = FullBlock<Self>;
}

pub fn lower_to_microcode(func: SSAFunction) {
    let lowered_func = lower_func(func);
    let register_lifetimes = lowered_func
        .blocks()
        .flat_map(|block| {
            empty()
                .chain(block.borrow().phis.iter().map(|phi| phi.dest.0))
                .chain(block.borrow().instructions.iter().map(|inst| inst.lhs.0))
                .collect_vec()
        })
        .map(|reg| (reg, find_liveness(&lowered_func, reg)))
        .collect::<HashMap<_, _>>();

    let register_conflicts = build_register_graph(&register_lifetimes);
    let register_allocation = color_registers(&register_conflicts, 2);

    let spilled_pos = RefCell::new(HashMap::new());

    // todo: handle writebacks to spilled
    // todo: handle multiple temps due to spills

    let read_register =
        |vreg, prelude: &mut Vec<Instruction<AllocatedMicrocodeConfig>>| match register_allocation
            [&vreg]
        {
            RegisterAllocation::Register(reg) => reg,
            RegisterAllocation::Spilled => {
                let out = PhysicalRegister { index: 0 };
                let next_offset = spilled_pos.borrow_mut().len() as u8;
                let index = *spilled_pos.borrow_mut().entry(vreg).or_insert(next_offset);
                prelude.push(Instruction {
                    lhs: out,
                    rhs: instructions::LoweredInstructionRHS::LoadRegister(index),
                });
                out
            }
        };

    let allocated_func = lower(
        lowered_func,
        |_, _blocks, inst| {
            let mut prelude = vec![];
            let rhs = inst
                .rhs
                .allocate_registers(|reg| read_register(reg, &mut prelude));
            let lhs = read_register(inst.lhs.0, &mut prelude);
            prelude.push(Instruction { lhs, rhs });
            prelude
        },
        |_, blocks, jmp| {
            let mut prelude = vec![];
            let jmp = jmp
                .map_reg_block_types(
                    |reg| Some(read_register(*reg, &mut prelude)),
                    |x| blocks.get(&x.as_key()).cloned(),
                )
                .unwrap();
            (prelude, jmp)
        },
        |lvalue| read_register(lvalue.0, &mut vec![]), // fixme spills
        |rvalue| read_register(rvalue, &mut vec![]),   // fixme spills
    );

    for block in allocated_func.blocks() {
        println!("{}", block.borrow());
    }}
