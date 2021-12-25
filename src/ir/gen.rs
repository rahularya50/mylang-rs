use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{bail, Context, Result};

use super::core_structs::{Block, Function, VirtualRegister};
use super::instructions::{Instruction, InstructionRHS, JumpInstruction};
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

pub struct LoopContext {
    loop_start: Rc<RefCell<Block>>,
    loop_break: Rc<RefCell<Block>>,
}

pub fn gen_expr<'a, 'b>(
    expr: &mut Expr,
    func: &mut Function,
    frame: &'b mut Frame<'a>,
    loops: &mut Vec<LoopContext>,
    mut block: Rc<RefCell<Block>>,
) -> Result<(Option<VirtualRegister>, Rc<RefCell<Block>>)> {
    Ok(match expr {
        Expr::VarDecl { name, value } => {
            if frame.lookup(name).is_some() {
                // this is a language-level requirement, not a limitation of the codegen
                bail!("variable shadowing is not permitted")
            } else {
                let (reg, block) = gen_expr(value, func, frame, loops, block)?;
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
            let dst = frame
                .lookup(name)
                .context("cannot assign to undeclared variable")?;
            let (src, block) = gen_expr(value, func, frame, loops, block)?;
            block.borrow_mut().instructions.push(Instruction::new(
                dst,
                InstructionRHS::Move {
                    src: src.context("cannot use a statement as the RHS of an assignment")?,
                },
            ));
            (None, block)
        }
        Expr::ArithOp {
            operator,
            arg1,
            arg2,
        } => {
            let (arg1, block) = gen_expr(arg1, func, frame, loops, block)?;
            let (arg2, block) = gen_expr(arg2, func, frame, loops, block)?;
            let out = func.new_reg();
            block.borrow_mut().instructions.push(Instruction::new(
                out,
                InstructionRHS::ArithmeticOperation {
                    operator: *operator,
                    arg1: arg1.context("cannot pass a statement as an argument")?,
                    arg2: arg2.context("cannot pass a statement as an argument")?,
                },
            ));
            (Some(out), block)
        }
        Expr::Block(exprs) => {
            let mut out = None;
            for expr in exprs.iter_mut() {
                let out_tmp;
                (out_tmp, block) = gen_expr(expr, func, frame, loops, block)?;
                out = Some(out_tmp);
            }
            (
                out.context("expr blocks must have at least one expression")?,
                block,
            )
        }
        Expr::IfElse { pred, conseq, alt } => {
            let (test, block) = gen_expr(pred, func, frame, loops, block)?;

            let conseq_block = Block::new_rc(func);
            let mut conseq_frame = frame.new_child();

            let alt_block = Block::new_rc(func);
            let mut alt_frame = frame.new_child();

            let jump = JumpInstruction::BranchIfElseZero {
                pred: test.context("cannot use a statement as the predicate of a conditional")?,
                conseq: conseq_block.clone(),
                alt: alt_block.clone(),
            };

            block.borrow_mut().exit = Some(jump);

            let (conseq_reg, conseq_block) =
                gen_expr(conseq, func, &mut conseq_frame, loops, conseq_block)?;
            let (alt_reg, alt_block) = gen_expr(alt, func, &mut alt_frame, loops, alt_block)?;

            let out = if let (Some(conseq_reg), Some(alt_reg)) = (conseq_reg, alt_reg) {
                let out = func.new_reg();
                conseq_block
                    .borrow_mut()
                    .instructions
                    .push(Instruction::new(
                        out,
                        InstructionRHS::Move { src: conseq_reg },
                    ));
                alt_block
                    .borrow_mut()
                    .instructions
                    .push(Instruction::new(out, InstructionRHS::Move { src: alt_reg }));
                Some(out)
            } else {
                None
            };

            let new_block = Block::new_rc(func);
            conseq_block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });
            alt_block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });
            (out, new_block)
        }
        Expr::IntegerLiteral(value) => {
            let out = func.new_reg();
            block.borrow_mut().instructions.push(Instruction::new(
                out,
                InstructionRHS::LoadIntegerLiteral { value: *value },
            ));
            (Some(out), block)
        }
        Expr::Noop => (None, block),
        Expr::Loop(body) => {
            let loop_start_block = Block::new_rc(func);
            let mut inner_frame = frame.new_child();

            block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: loop_start_block.clone(),
            });

            let new_block = Block::new_rc(func);

            loops.push(LoopContext {
                loop_start: loop_start_block.clone(),
                loop_break: new_block.clone(),
            });

            let (_, loop_final_block) = gen_expr(
                body,
                func,
                &mut inner_frame,
                loops,
                loop_start_block.clone(),
            )?;

            loops.pop().unwrap();

            loop_final_block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: loop_start_block,
            });

            (None, new_block)
        }
        Expr::Break => {
            let LoopContext { loop_break, .. } =
                loops.last().context("cannot break outside a loop")?;
            block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: loop_break.clone(),
            });
            (None, Block::new_rc(func))
        }
        Expr::Continue => {
            let LoopContext { loop_start, .. } =
                loops.last().context("cannot continue outside a loop")?;
            block.borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: loop_start.clone(),
            });
            (None, Block::new_rc(func))
        }
    })
}
