use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::empty;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use anyhow::{bail, Context, Result};
use itertools::Itertools;

use crate::semantics::{Expr, Operator};

#[derive(Debug)]
pub struct Function {
    reg_counter: u16,
    block_counter: u16,
    start_block: Rc<BlockRef>,
    pub blocks: Vec<Rc<BlockRef>>,
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for block in &self.blocks {
            writeln!(f, "{}", (***block).borrow())?;
        }
        Ok(())
    }
}

impl Function {
    fn new() -> Self {
        let start_block = Rc::new(BlockRef(RefCell::new(Block::new_with_index(0))));
        Function {
            reg_counter: 0,
            block_counter: 0,
            start_block: start_block.clone(),
            blocks: vec![start_block],
        }
    }

    fn new_reg(&mut self) -> VirtualRegisterLValue {
        self.reg_counter += 1;
        VirtualRegisterLValue(VirtualRegister {
            index: self.reg_counter,
        })
    }
}

struct Frame<'a> {
    symbol_table: HashMap<String, VirtualRegister>,
    parent: Option<&'a Frame<'a>>,
}

impl<'a> Frame<'a> {
    fn new() -> Self {
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

pub struct BlockRef(RefCell<Block>);

impl Deref for BlockRef {
    type Target = RefCell<Block>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for BlockRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockRef")
            .field("debug_index", &(**self).borrow().debug_index)
            .finish()
    }
}

impl BlockRef {
    fn new_rc(func: &mut Function) -> Rc<Self> {
        func.block_counter += 1;
        let out = Rc::new(Self(RefCell::new(Block::new_with_index(
            func.block_counter,
        ))));
        func.blocks.push(out.clone());
        out
    }
}

#[derive(Debug)]
pub struct Block {
    debug_index: u16,
    preds: Box<[Weak<BlockRef>]>,
    phis: Box<[Phi]>,
    instructions: Vec<Instruction>,
    exit: Option<JumpInstruction>,
}

impl Block {
    fn new_with_index(debug_index: u16) -> Self {
        Block {
            debug_index,
            preds: vec![].into_boxed_slice(),
            phis: vec![].into_boxed_slice(),
            instructions: vec![],
            exit: None,
        }
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "block {} (preds=", self.debug_index)?;
        write!(
            f,
            "{}",
            self.preds
                .iter()
                .map(|pred| (**pred.upgrade().unwrap()).borrow().debug_index)
                .join(",")
        )?;
        writeln!(f, ")")?;
        for phi in self.phis.iter() {
            writeln!(f, "{phi}")?;
        }
        for inst in &self.instructions {
            writeln!(f, "{inst}")?;
        }
        if let Some(exit) = &self.exit {
            writeln!(f, "{exit}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VirtualRegister {
    index: u16,
}

impl Display for VirtualRegister {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.index)
    }
}

#[derive(Debug)]
pub struct VirtualRegisterLValue(VirtualRegister);

impl Display for VirtualRegisterLValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0.index)
    }
}

#[derive(Debug)]
pub struct Phi {
    srcs: Box<[(VirtualRegister, Weak<BlockRef>)]>,
    dest: VirtualRegisterLValue,
}

impl Phi {
    fn new(
        srcs: impl IntoIterator<Item = (VirtualRegister, Weak<BlockRef>)>,
        dest: VirtualRegisterLValue,
    ) -> Self {
        Phi {
            srcs: srcs.into_iter().collect(),
            dest,
        }
    }
}

impl Display for Phi {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = phi({})",
            self.dest,
            self.srcs
                .iter()
                .map(|(reg, block)| {
                    format!(
                        "{} from block {}",
                        reg,
                        (**(block.upgrade().unwrap())).borrow().debug_index
                    )
                })
                .join(", ")
        )
    }
}

#[derive(Debug)]
pub enum Instruction {
    ArithmeticOperation {
        operator: Operator,
        arg1: VirtualRegister,
        arg2: VirtualRegister,
        out: VirtualRegisterLValue,
    },
    LoadIntegerLiteral {
        value: i64,
        out: VirtualRegisterLValue,
    },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::ArithmeticOperation {
                operator,
                arg1,
                arg2,
                out,
            } => {
                write!(f, "{out} = {arg1} {operator:?} {arg2}")
            }
            Instruction::LoadIntegerLiteral { value, out } => {
                write!(f, "{out} = {value}")
            }
        }
    }
}

#[derive(Debug)]
pub enum JumpInstruction {
    BranchIfElseZero {
        pred: VirtualRegister,
        conseq: Rc<BlockRef>,
        alt: Rc<BlockRef>,
    },
    UnconditionalJump {
        dest: Rc<BlockRef>,
    },
}

impl Display for JumpInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JumpInstruction::BranchIfElseZero { pred, conseq, alt } => {
                write!(
                    f,
                    "if {}==0 branchto {} else {}",
                    pred,
                    (***conseq).borrow().debug_index,
                    (***alt).borrow().debug_index
                )
            }
            JumpInstruction::UnconditionalJump { dest } => {
                write!(f, "jumpto {}", (***dest).borrow().debug_index)
            }
        }
    }
}

fn gen_expr_ssa<'a, 'b>(
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
                let (out_tmp, block_tmp) = gen_expr_ssa(expr, func, frame, block)?;
                out = Some(out_tmp);
                block = block_tmp;
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
            let mut phis = vec![Phi::new(
                [
                    (conseq_reg, Rc::downgrade(&conseq_block)),
                    (alt_reg, Rc::downgrade(&alt_block)),
                ],
                out,
            )];

            let conseq_frame = conseq_frame.symbol_table;
            let alt_frame = alt_frame.symbol_table;

            for var in empty()
                .chain(conseq_frame.keys())
                .chain(alt_frame.keys())
                .dedup()
            {
                // verify that this var is available in parent environment
                if let Some(parent_reg) = frame.lookup(var) {
                    let out @ VirtualRegisterLValue(out_ref) = func.new_reg();
                    phis.push(Phi::new(
                        [(&conseq_frame, &conseq_block), (&alt_frame, &alt_block)].map(
                            |(child_frame, block)| {
                                (
                                    if let Some(child_reg) = child_frame.get(var) {
                                        *child_reg
                                    } else {
                                        parent_reg
                                    },
                                    Rc::downgrade(block),
                                )
                            },
                        ),
                        out,
                    ));
                    frame.assoc(var.to_string(), out_ref);
                }
            }

            let new_block = BlockRef::new_rc(func);

            (**conseq_block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });
            (**alt_block).borrow_mut().exit = Some(JumpInstruction::UnconditionalJump {
                dest: new_block.clone(),
            });

            (**new_block).borrow_mut().phis = phis.into_boxed_slice();
            (**new_block).borrow_mut().preds =
                vec![Rc::downgrade(&conseq_block), Rc::downgrade(&alt_block)].into_boxed_slice();

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

pub fn gen_ssa(expr: &Expr) -> Result<Function> {
    let mut func = Function::new();
    let mut frame = Frame::new();
    let start_block = func.start_block.clone();
    gen_expr_ssa(expr, &mut func, &mut frame, start_block)?;
    Ok(func)
}
