use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::Itertools;

use super::register_liveness::{ConsumingPosition, RegisterLiveness};
use crate::backend::register_liveness::DefiningPosition;
use crate::ir::VirtualRegister;
use crate::utils::rcequality::RcEquality;

type RegisterLifetimeLookup<BType> =
    HashMap<RcEquality<Rc<RefCell<BType>>>, RegisterLiveness<BType>>;

fn lifetimes_overlap<BType>(
    lifetime1: &RegisterLiveness<BType>,
    lifetime2: &RegisterLiveness<BType>,
) -> bool {
    match (&lifetime1.until_index, &lifetime2.until_index) {
        (ConsumingPosition::Phi(phi1), ConsumingPosition::Phi(phi2)) => {
            assert!(lifetime1.since_index == DefiningPosition::Before);
            assert!(lifetime2.since_index == DefiningPosition::Before);
            // two registers only used in a phi block overlap
            // iff they are from the same source
            phi1.src == phi2.src
        }
        _ => {
            #[allow(clippy::needless_bool)]
            if lifetime1.since_index >= lifetime2.until_index
                || lifetime2.since_index >= lifetime1.until_index
            {
                // reg1 is only used after reg2 is dropped
                false
            } else {
                true
            }
        }
    }
}

pub fn build_register_graph<RType>(
    register_lifetimes: &HashMap<VirtualRegister, RegisterLifetimeLookup<RType>>,
) -> HashMap<VirtualRegister, HashSet<VirtualRegister>> {
    let mut out = HashMap::<_, HashSet<_>>::new();
    for (reg1, reg1_lifetimes) in register_lifetimes {
        for (reg2, reg2_lifetimes) in register_lifetimes {
            for (block_ref, reg1_lifetime) in reg1_lifetimes {
                if let Some(reg2_lifetime) = reg2_lifetimes.get(block_ref) {
                    // both reg1 and reg2 are alive in the same block
                    if lifetimes_overlap(reg1_lifetime, reg2_lifetime) {
                        // reg1 is defined before reg2 dies, and lives until after reg2 is created, hence overlap
                        // or, vice-versa
                        out.entry(*reg1).or_default().insert(*reg2);
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct PhysicalRegister {
    pub index: u8,
}

pub enum RegisterAllocation {
    Register(PhysicalRegister),
    Spilled,
}

// see Section 6 of https://www.cs.cmu.edu/~fp/courses/15411-f13/lectures/03-regalloc.pdf
pub fn color_registers(
    graph: &HashMap<VirtualRegister, HashSet<VirtualRegister>>,
    num_registers: usize,
) -> HashMap<VirtualRegister, RegisterAllocation> {
    let mut weights = HashMap::<VirtualRegister, i32>::new();
    let mut remaining_vertices = HashSet::<_>::from_iter(graph.keys());
    let mut ordering = vec![];
    while let Some(next_vert) = remaining_vertices
        .iter()
        .max_by_key(|reg| weights.get(reg).unwrap_or(&0))
        .cloned()
    {
        ordering.push(next_vert);
        remaining_vertices.remove(next_vert);
        for vert in &graph[next_vert] {
            *weights.entry(*vert).or_default() += 1;
        }
    }

    let mut coloring = HashMap::new();
    let mut colorcounts = HashMap::new();

    for reg in ordering {
        'indices: for index in 0.. {
            let candidate_reg = PhysicalRegister { index };
            for neighbor in &graph[reg] {
                if let Some(color) = coloring.get(neighbor) {
                    if *color == candidate_reg {
                        // oh no
                        continue 'indices;
                    }
                }
            }
            coloring.insert(*reg, candidate_reg);
            *colorcounts.entry(candidate_reg).or_insert(0) += 1;
        }
    }

    let spilled_colors = HashSet::<_>::from_iter(
        colorcounts
            .keys()
            .sorted_by_key(|key| colorcounts[key])
            .rev()
            .take(colorcounts.len() - num_registers),
    );

    coloring
        .into_iter()
        .map(|(vreg, color)| {
            if spilled_colors.contains(&color) {
                (vreg, RegisterAllocation::Spilled)
            } else {
                (vreg, RegisterAllocation::Register(color))
            }
        })
        .collect()
}
