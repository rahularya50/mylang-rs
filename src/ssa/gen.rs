use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use itertools::Itertools;

use super::core_structs::{BlockRef, Function, Phi, VirtualRegister, VirtualRegisterLValue};
use super::instructions::{Instruction, JumpInstruction};
use crate::semantics::Expr;

pub struct Frame<'a> {
    symbol_table: HashMap<String, VirtualRegister>,
    parent: Option<&'a Frame<'a>>,
}

impl<'a> Frame<'a> {
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
            parent: None,
        }
    }

    fn new_child(&'a self) -> Frame {
        Self {
            symbol_table: HashMap::new(),
            parent: Some(self),
        }
    }

    fn lookup(&self, name: &str) -> Option<VirtualRegister> {
        self.symbol_table
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|p| p.lookup(name)))
    }

    fn assoc(&mut self, name: String, reg: VirtualRegister) {
        self.symbol_table.insert(name, reg);
    }
}

fn unified_block_phis(
    func: &mut Function,
    parent_frame: &mut Frame,
    preds: &[(Rc<BlockRef>, HashMap<String, VirtualRegister>)],
) -> Vec<Phi> {
    preds
        .iter()
        .flat_map(|(_, child_symbols)| child_symbols.keys())
        .dedup()
        .filter_map(|var| {
            parent_frame.lookup(var).map(|parent_reg| {
                let out @ VirtualRegisterLValue(out_ref) = func.new_reg();
                parent_frame.assoc(var.to_string(), out_ref);
                Phi::new(
                    preds.iter().map(|(block, child_symbols)| {
                        (
                            if let Some(child_reg) = child_symbols.get(var) {
                                *child_reg
                            } else {
                                parent_reg
                            },
                            Rc::downgrade(block),
                        )
                    }),
                    out,
                )
            })
        })
        .collect()
}

fn find_loop_variables(expr: &mut Expr) -> Vec<String> {
    let process = |children: &mut [&mut Expr]| {
        children
            .iter_mut()
            .flat_map(|x| find_loop_variables(x))
            .collect::<Vec<_>>()
    };

    match &mut *expr {
        Expr::VarAccess(name) => return vec![name.to_string()],
        Expr::Loop {
            used_variables: Some(used_variables),
            ..
        } => used_variables.clone(),
        Expr::Loop {
            body,
            used_variables,
            ..
        } => {
            let out = process(&mut [body]);
            *used_variables = Some(out.clone());
            out
        }
        Expr::VarDecl { value, .. } | Expr::VarAssign { value, .. } => process(&mut [value]),
        Expr::ArithOp { arg1, arg2, .. } => process(&mut [arg1, arg2]),
        Expr::Block(exprs) => process(&mut exprs.iter_mut().collect::<Vec<_>>()),
        Expr::IfElse { pred, conseq, alt } => process(&mut [pred, conseq, alt]),
        Expr::Break | Expr::IntegerLiteral(_) | Expr::Noop => vec![],
    }
}

pub fn gen_expr_ssa<'a, 'b>(
    expr: &mut Expr,
    func: &mut Function,
    frame: &'b mut Frame<'a>,
    mut block: Rc<BlockRef>,
) -> Result<(Option<VirtualRegister>, Rc<BlockRef>)> {
    Ok(match expr {
        Expr::VarDecl { name, value } => {
            if frame.lookup(name).is_some() {
                bail!("variable shadowing is not permitted")
            } else {
                let (reg, block) = gen_expr_ssa(value, func, frame, block)?;
                frame.assoc(
                    name.to_string(),
                    reg.context("cannot use a statement as the RHS of a declaration")?,
                );
                (reg, block)
            }
        }
        Expr::VarAccess(name) => (
            Some(frame.lookup(name).context("variable not found in scope")?),
            block,
        ),
        Expr::VarAssign { name, value } => {
            frame
                .lookup(name)
                .context("cannot assign to undeclared variable")?;
            let (reg, block) = gen_expr_ssa(value, func, frame, block)?;
            frame.assoc(
                name.to_string(),
                reg.context("cannot use a statement as the RHS of an assignment")?,
            );
            (reg, block)
        }
        Expr::ArithOp {
            operator,
            arg1,
            arg2,
        } => {
            let (arg1, block) = gen_expr_ssa(arg1, func, frame, block)?;
            let (arg2, block) = gen_expr_ssa(arg2, func, frame, block)?;
            let out @ VirtualRegisterLValue(out_ref) = func.new_reg();
            (**block)
                .borrow_mut()
                .instructions
                .push(Instruction::ArithmeticOperation {
                    operator: *operator,
                    arg1: arg1.context("cannot pass a statement as an argument")?,
                    arg2: arg2.context("cannot pass a statement as an argument")?,
                    out,
                });
            (Some(out_ref), block)
        }
        Expr::Block(exprs) => {
            let mut out = None;
            for expr in exprs.iter_mut() {
                let out_tmp;
                (out_tmp, block) = gen_expr_ssa(expr, func, frame, block)?;
                out = Some(out_tmp);
            }
            (
                out.context("expr blocks must have at least one expression")?,
                block,
            )
        }
        Expr::IfElse { pred, conseq, alt } => {
            let (test, block) = gen_expr_ssa(pred, func, frame, block)?;

            let conseq_block = BlockRef::new_rc(func);
            let mut conseq_frame = frame.new_child();

            let alt_block = BlockRef::new_rc(func);
            let mut alt_frame = frame.new_child();

            let jump = JumpInstruction::BranchIfElseZero {
                pred: test.context("cannot use a statement as the predicate of a conditional")?,
                conseq: conseq_block.clone(),
                alt: alt_block.clone(),
            };

            (**block).borrow_mut().exit = Some(jump);
            (**conseq_block).borrow_mut().preds = vec![Rc::downgrade(&block)].into_boxed_slice();
            (**alt_block).borrow_mut().preds = vec![Rc::downgrade(&block)].into_boxed_slice();

            let (conseq_reg, conseq_block) =
                gen_expr_ssa(conseq, func, &mut conseq_frame, conseq_block)?;

            let (alt_reg, alt_block) = gen_expr_ssa(alt, func, &mut alt_frame, alt_block)?;

            let preds = [
                (conseq_block.clone(), conseq_frame.symbol_table),
                (alt_block.clone(), alt_frame.symbol_table),
            ];

            let mut phis = unified_block_phis(func, frame, &preds);

            let out_ref = if let (Some(conseq_reg), Some(alt_reg)) = (conseq_reg, alt_reg) {
                let out @ VirtualRegisterLValue(out_ref) = func.new_reg();
                phis.push(Phi::new(
                    [
                        (conseq_reg, Rc::downgrade(&conseq_block)),
                        (alt_reg, Rc::downgrade(&alt_block)),
                    ],
                    out,
                ));
                Some(out_ref)
            } else {
                None
            };

            let new_block = BlockRef::new_rc(func);

            (**new_block).borrow_mut().phis = phis.into_boxed_slice();
            (**new_block).borrow_mut().preds =
                vec![Rc::downgrade(&conseq_block), Rc::downgrade(&alt_block)].into_boxed_slice();

            (**conseq_block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });
            (**alt_block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });

            (out_ref, new_block)
        }
        Expr::IntegerLiteral(value) => {
            let out @ VirtualRegisterLValue(out_ref) = func.new_reg();
            (**block)
                .borrow_mut()
                .instructions
                .push(Instruction::LoadIntegerLiteral { value: *value, out });
            (Some(out_ref), block)
        }
        Expr::Noop => (None, block),
        Expr::Loop {
            body,
            used_variables,
        } => {
            let inner_block = BlockRef::new_rc(func);
            let mut inner_frame = frame.new_child();

            (**block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: inner_block.clone(),
            });

            let loop_variables = find_loop_variables(body);
            *used_variables = Some(loop_variables.clone());

            let (loop_phis, loop_phi_variables) = loop_variables
                .iter()
                .filter_map(|var| {
                    frame.lookup(var).map(|parent_reg| {
                        // the parent frame contains this register, so we can safely optimistically phi it!
                        let loop_start_reg @ VirtualRegisterLValue(loop_start_reg_ref) =
                            func.new_reg();
                        let loop_end_reg @ VirtualRegisterLValue(loop_end_reg_ref) = func.new_reg();
                        inner_frame.assoc(var.to_string(), loop_start_reg_ref);
                        (
                            Phi::new(
                                [
                                    (parent_reg, Rc::downgrade(&block)),
                                    (loop_end_reg_ref, Rc::downgrade(&inner_block)),
                                ],
                                loop_start_reg,
                            ),
                            (var, loop_start_reg_ref, loop_end_reg),
                        )
                    })
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();

            (**inner_block).borrow_mut().preds =
                vec![Rc::downgrade(&block), Rc::downgrade(&inner_block)].into_boxed_slice();
            (**inner_block).borrow_mut().phis = loop_phis.into_boxed_slice();

            for (var, start_reg, _) in loop_phi_variables.iter() {
                inner_frame.assoc(var.to_string(), *start_reg)
            }

            let (_, inner_block) = gen_expr_ssa(body, func, &mut inner_frame, inner_block)?;

            for (var, _, end_reg) in loop_phi_variables {
                (**inner_block)
                    .borrow_mut()
                    .instructions
                    .push(Instruction::Move {
                        src: inner_frame.lookup(var).unwrap(),
                        out: end_reg,
                    })
            }

            (**inner_block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: inner_block.clone(),
            });

            let preds = [(block.clone(), frame.symbol_table.clone())];

            let phis = unified_block_phis(func, frame, &preds);

            let new_block = BlockRef::new_rc(func);

            (**new_block).borrow_mut().phis = phis.into_boxed_slice();
            (**new_block).borrow_mut().preds =
                vec![Rc::downgrade(&block), Rc::downgrade(&inner_block)].into_boxed_slice();

            (None, new_block)
        }
        Expr::Break => todo!(),
    })
}
