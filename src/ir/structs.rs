use std::cell::RefCell;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::{Rc, Weak};

use itertools::Itertools;

use super::instructions::{Instruction, JumpInstruction};

#[derive(Debug)]
pub struct Function {
    var_counter: u16,
    reg_counter: u16,
    block_counter: u16,
    pub start_block: BlockRef,
    pub blocks: Vec<Weak<RefCell<Block>>>,
}

impl Function {
    pub fn new() -> Self {
        let start_block = Rc::new(RefCell::new(Block::new_with_index(0)));
        Function {
            var_counter: 0,
            reg_counter: 0,
            block_counter: 0,
            start_block: start_block.clone(),
            blocks: vec![Rc::downgrade(&start_block)],
        }
    }

    pub fn new_var(&mut self) -> VirtualVariable {
        self.var_counter += 1;
        VirtualVariable {
            index: self.var_counter,
        }
    }

    pub fn new_reg(&mut self) -> VirtualRegisterLValue {
        self.reg_counter += 1;
        VirtualRegisterLValue(VirtualRegister {
            index: self.reg_counter,
        })
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for block in &self.blocks {
            if let Some(block) = block.upgrade() {
                writeln!(f, "{}", block.borrow())?
            };
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block {
    pub(super) debug_index: u16,
    pub instructions: Vec<Instruction>,
    pub exit: JumpInstruction<VirtualVariable>,
}

pub type BlockRef = Rc<RefCell<Block>>;

impl Block {
    fn new_with_index(debug_index: u16) -> Self {
        Block {
            debug_index,
            instructions: vec![],
            exit: JumpInstruction::Ret,
        }
    }

    pub fn new_rc(func: &mut Function) -> Rc<RefCell<Self>> {
        func.block_counter += 1;
        let out = Rc::new(RefCell::new(Self::new_with_index(func.block_counter)));
        func.blocks.push(Rc::downgrade(&out));
        out
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "block {}", self.debug_index)?;
        for inst in &self.instructions {
            writeln!(f, "{inst}")?;
        }
        writeln!(f, "{}", self.exit)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SSABlock {
    pub(super) debug_index: u16,
    pub preds: Box<[Weak<RefCell<Block>>]>,
    pub phis: Box<[Phi]>,
    pub instructions: Vec<Instruction>,
    pub exit: Option<JumpInstruction<VirtualRegister>>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct VirtualRegister {
    index: u16,
}

impl Display for VirtualRegister {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "var{}", self.index)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct VirtualVariable {
    index: u16,
}

impl Display for VirtualVariable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "var{}", self.index)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct VirtualRegisterLValue(pub VirtualRegister);

impl Display for VirtualRegisterLValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0.index)
    }
}

#[derive(Debug)]
pub struct Phi {
    pub srcs: Vec<(VirtualRegister, Weak<RefCell<Block>>)>,
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
