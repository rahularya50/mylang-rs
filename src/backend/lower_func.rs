use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::ir::{
    CfgConfig, FullBlock, Function, Instruction, JumpInstruction, Phi, RegisterLValue,
};
use crate::utils::rcequality::{RcDereferencable, RcEquality};

pub fn lower<
    Conf: CfgConfig<BlockType = FullBlock<Conf>>,
    NewConf: CfgConfig<BlockType = FullBlock<NewConf>>,
    InstIter: IntoIterator<Item = Instruction<NewConf>>,
    JmpInstIter: IntoIterator<Item = Instruction<NewConf>>,
>(
    func: Function<Conf>,
    mut map_inst: impl FnMut(
        &mut Function<NewConf>,
        &HashMap<RcEquality<Rc<RefCell<FullBlock<Conf>>>>, Rc<RefCell<FullBlock<NewConf>>>>,
        Instruction<Conf>,
    ) -> InstIter,
    mut map_jump: impl FnMut(
        &mut Function<NewConf>,
        &HashMap<RcEquality<Rc<RefCell<FullBlock<Conf>>>>, Rc<RefCell<FullBlock<NewConf>>>>,
        JumpInstruction<Conf>,
    ) -> (JmpInstIter, JumpInstruction<NewConf>),
    mut map_lvalues: impl FnMut(Conf::LValue) -> NewConf::LValue,
    mut map_rvalues: impl FnMut(Conf::RValue) -> NewConf::RValue,
) -> Function<NewConf> {
    let mut block_lookup = HashMap::new();
    for block in func.blocks() {
        block_lookup.insert(block.into(), Rc::new(RefCell::new(FullBlock::default())));
    }
    let new_start_block = block_lookup[&func.start_block.as_key()].clone();
    let old_blocks = func.blocks().collect_vec();
    let mut new_func = func.lower(new_start_block, vec![]);
    for block_ref in old_blocks {
        let out_block = block_lookup.get(&block_ref.as_key()).unwrap();
        let block = block_ref.take();
        let mut instructions = vec![];
        for inst in block.instructions {
            instructions.extend(map_inst(&mut new_func, &block_lookup, inst))
        }
        out_block.borrow_mut().debug_index = block.debug_index;
        out_block.borrow_mut().preds = block
            .preds
            .into_iter()
            .filter_map(|pred| pred.get_ref().upgrade())
            .map(|pred| Rc::downgrade(&block_lookup[&pred.as_key()]).into())
            .collect();
        out_block.borrow_mut().phis = block
            .phis
            .into_iter()
            .map(|phi| Phi {
                srcs: phi
                    .srcs
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            Rc::downgrade(&block_lookup[&k.get_ref().as_key()]).into(),
                            map_rvalues(v),
                        )
                    })
                    .collect(),
                dest: map_lvalues(phi.dest),
            })
            .collect();
        let (insts, new_jump) = map_jump(&mut new_func, &block_lookup, block.exit);
        instructions.extend(insts);
        out_block.borrow_mut().instructions = instructions;
        out_block.borrow_mut().exit = new_jump;
    }
    new_func.blocks = block_lookup.values().map(Rc::downgrade).collect_vec();
    new_func
}
