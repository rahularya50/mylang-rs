use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

use itertools::Itertools;

use super::instructions::{Instruction, JumpInstruction};
use crate::utils::rcequality::RcEquality;

#[derive(Debug)]
pub struct Function<RegType, BlockType> {
    reg_counter: u16,
    block_counter: Option<u16>,
    pub start_block: Rc<RefCell<BlockType>>,
    pub blocks: Vec<Weak<RefCell<BlockType>>>,
    _reg: PhantomData<RegType>,
}

impl<RegType, BlockType: BlockWithDebugIndex> Function<RegType, BlockType> {
    pub fn new() -> Self {
        let start_block = Rc::new(RefCell::new(BlockType::new_with_index(0)));
        Self {
            reg_counter: 0,
            block_counter: None,
            start_block: start_block.clone(),
            blocks: vec![Rc::downgrade(&start_block)],
            _reg: PhantomData,
        }
    }

    pub fn new_block(&mut self) -> Rc<RefCell<BlockType>> {
        let next_counter = self.block_counter.map(|x| x + 1).unwrap_or_default();
        let out = Rc::new(RefCell::new(BlockType::new_with_index(next_counter)));
        self.blocks.push(Rc::downgrade(&out));
        if self.block_counter.is_none() {
            self.start_block = out.clone();
        }
        self.block_counter = Some(next_counter);
        out
    }

    pub fn blocks(&self) -> impl Iterator<Item = Rc<RefCell<BlockType>>> + '_ {
        self.blocks.iter().filter_map(std::rc::Weak::upgrade)
    }

    pub fn clear_dead_blocks(&mut self) {
        self.blocks.drain_filter(|block| block.upgrade().is_none());
    }
}

impl<RegType: RegisterLValue, BlockType> Function<RegType, BlockType> {
    pub fn new_reg(&mut self) -> RegType {
        self.reg_counter += 1;
        RegType::new(self.reg_counter)
    }
}

impl<RegType, BlockType: Display + BlockWithDebugIndex> Display for Function<RegType, BlockType> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "start: {}\n",
            self.start_block.borrow().get_debug_index()
        )?;
        for block in &self.blocks {
            if let Some(block) = block.upgrade() {
                writeln!(f, "{}", block.borrow())?;
            };
        }
        Ok(())
    }
}

pub trait BlockWithDebugIndex {
    fn new_with_index(debug_index: u16) -> Self;
    fn get_debug_index(&self) -> u16;
}

pub trait RegisterLValue {
    type RValue;
    fn new(index: u16) -> Self;
}

#[derive(Debug)]
pub struct Block {
    pub(super) debug_index: u16,
    pub instructions: Vec<Instruction<VirtualVariable>>,
    pub exit: JumpInstruction<VirtualVariable, Block>,
}

pub type BlockRef = Rc<RefCell<Block>>;

impl BlockWithDebugIndex for Block {
    fn new_with_index(debug_index: u16) -> Self {
        Self {
            debug_index,
            instructions: vec![],
            exit: JumpInstruction::Ret(None),
        }
    }

    fn get_debug_index(&self) -> u16 {
        self.debug_index
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
pub struct FullBlock<IType> {
    pub debug_index: u16,
    pub preds: HashSet<RcEquality<Weak<RefCell<Self>>>>,
    pub phis: Vec<Phi<IType>>,
    pub instructions: Vec<IType>,
    pub exit: JumpInstruction<VirtualRegister, Self>,
}

impl<IType> FullBlock<IType> {
    pub fn preds(&self) -> impl Iterator<Item = Rc<RefCell<Self>>> + '_ {
        self.preds
            .iter()
            .filter_map(|pred| pred.get_ref().upgrade())
    }
}

impl<IType> BlockWithDebugIndex for FullBlock<IType> {
    fn new_with_index(debug_index: u16) -> Self {
        Self {
            debug_index,
            preds: HashSet::new(),
            phis: vec![],
            instructions: vec![],
            exit: JumpInstruction::Ret(None),
        }
    }

    fn get_debug_index(&self) -> u16 {
        self.debug_index
    }
}

impl<IType> Default for FullBlock<IType> {
    fn default() -> Self {
        Self {
            debug_index: 0,
            preds: HashSet::new(),
            phis: vec![],
            instructions: vec![],
            exit: JumpInstruction::Ret(None),
        }
    }
}

impl<IType: Display> Display for FullBlock<IType> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "block {} (preds=[{}])",
            self.debug_index,
            self.preds()
                .map(|pred| format!("{}", pred.borrow().debug_index))
                .join(", ")
        )?;
        for phi in &self.phis {
            writeln!(f, "{phi}")?;
        }
        for inst in &self.instructions {
            writeln!(f, "{inst}")?;
        }
        writeln!(f, "{}", self.exit)?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct VirtualVariable {
    index: u16,
}

impl RegisterLValue for VirtualVariable {
    type RValue = Self;

    fn new(index: u16) -> Self {
        Self { index }
    }
}

impl Display for VirtualVariable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "var{}", self.index)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct VirtualRegister {
    index: u16,
}

impl Display for VirtualRegister {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.index)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct VirtualRegisterLValue(pub VirtualRegister);

impl RegisterLValue for VirtualRegisterLValue {
    type RValue = VirtualRegister;

    fn new(index: u16) -> Self {
        Self(VirtualRegister { index })
    }
}

impl Display for VirtualRegisterLValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0.index)
    }
}

#[derive(Debug)]
pub struct Phi<IType> {
    pub srcs: HashMap<RcEquality<Weak<RefCell<FullBlock<IType>>>>, VirtualRegister>,
    pub dest: VirtualRegisterLValue,
}

impl<IType> Display for Phi<IType> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = phi({})",
            self.dest,
            self.srcs
                .iter()
                .map(|(block, reg)| {
                    format!(
                        "{} from block {}",
                        reg,
                        block.get_ref().upgrade().unwrap().borrow().debug_index
                    )
                })
                .join(", ")
        )
    }
}
