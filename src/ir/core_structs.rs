use std::cell::RefCell;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::{Rc, Weak};

use itertools::Itertools;

use super::instructions::{Instruction, JumpInstruction};

#[derive(Debug)]
pub struct Function {
    reg_counter: u16,
    block_counter: u16,
    pub start_block: Rc<RefCell<Block>>,
    pub blocks: Vec<Rc<RefCell<Block>>>,
}

impl Function {
    pub fn new() -> Self {
        let start_block = Rc::new(RefCell::new(Block::new_with_index(0)));
        Function {
            reg_counter: 0,
            block_counter: 0,
            start_block: start_block.clone(),
            blocks: vec![start_block],
        }
    }

    pub fn new_reg(&mut self) -> VirtualRegister {
        self.reg_counter += 1;
        VirtualRegister {
            index: self.reg_counter,
        }
    }

    pub fn new_ssa_reg(&mut self) -> VirtualRegisterLValue {
        self.reg_counter += 1;
        VirtualRegisterLValue(VirtualRegister {
            index: self.reg_counter,
        })
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for block in &self.blocks {
            writeln!(f, "{}", block.borrow())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block {
    pub(super) debug_index: u16,
    pub instructions: Vec<Instruction>,
    pub exit: Option<JumpInstruction>,
}

impl Block {
    fn new_with_index(debug_index: u16) -> Self {
        Block {
            debug_index,
            instructions: vec![],
            exit: None,
        }
    }

    pub fn new_rc(func: &mut Function) -> Rc<RefCell<Self>> {
        func.block_counter += 1;
        let out = Rc::new(RefCell::new(Self::new_with_index(func.block_counter)));
        func.blocks.push(out.clone());
        out
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "block {}", self.debug_index)?;
        for inst in &self.instructions {
            writeln!(f, "{inst}")?;
        }
        if let Some(exit) = &self.exit {
            writeln!(f, "{exit}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SSABlock {
    pub(super) debug_index: u16,
    pub preds: Box<[Weak<RefCell<Block>>]>,
    pub phis: Box<[Phi]>,
    pub instructions: Vec<Instruction>,
    pub exit: Option<JumpInstruction>,
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
pub struct VirtualRegisterLValue(pub VirtualRegister);

impl Display for VirtualRegisterLValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0.index)
    }
}

#[derive(Debug)]
pub struct Phi {
    pub srcs: Box<[(VirtualRegister, Weak<RefCell<Block>>)]>,
    pub dest: VirtualRegisterLValue,
}

impl Phi {
    pub fn new(
        srcs: impl IntoIterator<Item = (VirtualRegister, Weak<RefCell<Block>>)>,
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
                        (block.upgrade().unwrap()).borrow().debug_index
                    )
                })
                .join(", ")
        )
    }
}
