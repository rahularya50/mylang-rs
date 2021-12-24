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

pub fn gen_expr_ssa<'a, 'b>(
    expr: &Expr,
    func: &mut Function,
    frame: &'b mut Frame<'a>,
    mut block: Rc<BlockRef>,
) -> Result<(VirtualRegister, Rc<BlockRef>)> {
    Ok(match expr {
        Expr::VarDecl { name, value } => {
            if frame.lookup(name).is_some() {
                bail!("variable shadowing is not permitted")
            } else {
                let (reg, block) = gen_expr_ssa(value, func, frame, block)?;
                frame.assoc(name.to_string(), reg);
                (reg, block)
            }
        }
        Expr::VarAccess(name) => (
            frame.lookup(name).context("variable not found in scope")?,
            block,
        ),
        Expr::VarAssign { name, value } => {
            frame
                .lookup(name)
                .context("cannot assign to undeclared variable")?;
            let (reg, block) = gen_expr_ssa(value, func, frame, block)?;
            frame.assoc(name.to_string(), reg);
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
                    arg1,
                    arg2,
                    out,
                });
            (out_ref, block)
        }
        Expr::Block(exprs) => {
            let mut out = None;
            for expr in exprs.iter() {
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
                pred: test,
                conseq: conseq_block.clone(),
                alt: alt_block.clone(),
            };

            (**block).borrow_mut().exit = Some(jump);
            (**conseq_block).borrow_mut().preds = vec![Rc::downgrade(&block)].into_boxed_slice();
            (**alt_block).borrow_mut().preds = vec![Rc::downgrade(&block)].into_boxed_slice();

            let (conseq_reg, conseq_block) =
                gen_expr_ssa(conseq, func, &mut conseq_frame, conseq_block)?;

            let (alt_reg, alt_block) = gen_expr_ssa(alt, func, &mut alt_frame, alt_block)?;

            let out @ VirtualRegisterLValue(out_ref) = func.new_reg();

            let preds = vec![
                (conseq_block.clone(), conseq_frame.symbol_table),
                (alt_block.clone(), alt_frame.symbol_table),
            ];

            let mut phis = unified_block_phis(func, frame, &preds);
            phis.push(Phi::new(
                [
                    (conseq_reg, Rc::downgrade(&conseq_block)),
                    (alt_reg, Rc::downgrade(&alt_block)),
                ],
                out,
            ));

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
            (out_ref, block)
        }
    })
}
