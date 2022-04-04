use std::collections::{HashMap, HashSet};
use std::mem::take;

use crate::ir::{
    Phi, SSAFunction, SSAInstruction, SSAInstructionRHS, SSAJumpInstruction, VirtualRegister,
};
use crate::semantics::{BinaryOperator, UnaryOperator};
use crate::utils::rcequality::RcDereferencable;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RegisterValue {
    Constant(i64),
    Variable,
}

const fn unify(a: Option<RegisterValue>, b: Option<RegisterValue>) -> Option<RegisterValue> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(RegisterValue::Constant(x)), Some(RegisterValue::Constant(y))) => {
            if x == y {
                Some(RegisterValue::Constant(x))
            } else {
                Some(RegisterValue::Variable)
            }
        }
        (_, Some(RegisterValue::Variable)) | (Some(RegisterValue::Variable), _) => {
            Some(RegisterValue::Variable)
        }
    }
}

fn evaluate(
    rhs: &SSAInstructionRHS,
    known_values: &HashMap<VirtualRegister, RegisterValue>,
) -> Option<i64> {
    let get_reg = |reg| match known_values[reg] {
        RegisterValue::Constant(val) => Some(val),
        RegisterValue::Variable => None,
    };
    Some(match rhs {
        SSAInstructionRHS::UnaryOperation {
            operator: UnaryOperator::Not,
            arg,
        } => !get_reg(arg)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::Xor,
            arg1,
            arg2,
        } => get_reg(arg1)? ^ get_reg(arg2)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::And,
            arg1,
            arg2,
        } => get_reg(arg1)? & get_reg(arg2)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::Add,
            arg1,
            arg2,
        } => get_reg(arg1)? + get_reg(arg2)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::Sub,
            arg1,
            arg2,
        } => get_reg(arg1)? - get_reg(arg2)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::Mul,
            arg1,
            arg2,
        } => get_reg(arg1)? * get_reg(arg2)?,
        SSAInstructionRHS::BinaryOperation {
            operator: BinaryOperator::Div,
            arg1: _,
            arg2: _,
        } => return None,
        SSAInstructionRHS::LoadIntegerLiteral { value } => *value,
        SSAInstructionRHS::Move { src } => get_reg(src)?,
        SSAInstructionRHS::ReadInput => return None,
        SSAInstructionRHS::ReadMemory(_) => return None,
    })
}

pub fn constant_folding(func: &mut SSAFunction) {
    let mut visited_blocks = HashSet::new();
    let mut known_values = HashMap::new();
    let mut blocks_to_explore = vec![func.start_block.clone()];
    while let Some(block_ref) = blocks_to_explore.pop() {
        let block = block_ref.borrow();

        let mut changed = false;

        for Phi { srcs, dest } in &block.phis {
            let val = srcs
                .values()
                .map(|src| known_values.get(src).copied())
                .reduce(unify)
                .flatten()
                .expect("phi srcs must be nonempty");
            if known_values.insert(dest.0, val) != Some(val) {
                changed = true;
            }
        }

        for inst in &block.instructions {
            let val = evaluate(&inst.rhs, &known_values)
                .map_or(RegisterValue::Variable, RegisterValue::Constant);
            if known_values.insert(inst.lhs.0, val) != Some(val) {
                changed = true;
            }
        }

        if changed {
            // we need to re-explore all blocks reachable from our current node, since stuff has changed
            // ideally we'd only re-explore blocks that read the affected registers, but this is good enough
            visited_blocks.drain();
        }

        let not_previously_visited = visited_blocks.insert(block_ref.as_key());

        if not_previously_visited {
            match &block.exit {
                SSAJumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                    match known_values[pred] {
                        RegisterValue::Constant(val) => {
                            if val == 0 {
                                blocks_to_explore.push(conseq.clone());
                            } else {
                                blocks_to_explore.push(alt.clone());
                            }
                        }
                        RegisterValue::Variable => {
                            blocks_to_explore.push(conseq.clone());
                            blocks_to_explore.push(alt.clone());
                        }
                    }
                }
                SSAJumpInstruction::UnconditionalJump { dest } => {
                    blocks_to_explore.push(dest.clone());
                }
                SSAJumpInstruction::Ret(_) => {}
            }
        }
    }

    // now, replace constants!
    for block in func.blocks() {
        let mut block = block.borrow_mut();
        let mut phi_assigns = vec![];
        block
            .phis
            .drain_filter(|phi| {
                matches!(
                    known_values.get(&phi.dest.0).copied(),
                    Some(RegisterValue::Constant(_))
                )
            })
            .for_each(|phi| match known_values[&phi.dest.0] {
                RegisterValue::Constant(value) => phi_assigns.push(SSAInstruction::new(
                    phi.dest,
                    SSAInstructionRHS::LoadIntegerLiteral { value },
                )),
                RegisterValue::Variable => {
                    unreachable!("unexpected pattern mismatch, phi var must have constant val")
                }
            });
        for inst in &mut block.instructions {
            if let Some(RegisterValue::Constant(value)) = known_values.get(&inst.lhs.0).copied() {
                inst.rhs = SSAInstructionRHS::LoadIntegerLiteral { value }
            }
        }
        phi_assigns.extend(take(&mut block.instructions));
        block.instructions = phi_assigns;
        if let SSAJumpInstruction::BranchIfElseZero { pred, conseq, alt } = &block.exit {
            if let Some(RegisterValue::Constant(val)) = known_values.get(pred).copied() {
                if val == 0 {
                    block.exit = SSAJumpInstruction::UnconditionalJump {
                        dest: conseq.clone(),
                    };
                } else {
                    block.exit = SSAJumpInstruction::UnconditionalJump { dest: alt.clone() };
                }
            }
        }
    }
}
