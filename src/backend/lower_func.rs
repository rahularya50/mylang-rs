use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use itertools::Itertools;

use crate::ir::{FullBlock, Function, JumpInstruction, Phi, RegisterLValue};
use crate::utils::rcequality::{RcDereferencable, RcEquality};

pub fn lower<
    RegType: RegisterLValue,
    IType,
    NewRegType: RegisterLValue,
    NewIType,
    InstIter: IntoIterator<Item = NewIType>,
    JmpInstIter: IntoIterator<Item = NewIType>,
    InstMapper: FnMut(
        &mut Function<NewRegType, FullBlock<NewIType, NewRegType>>,
        &HashMap<
            RcEquality<Rc<RefCell<FullBlock<IType, RegType>>>>,
            Rc<RefCell<FullBlock<NewIType, NewRegType>>>,
        >,
        IType,
    ) -> InstIter,
    JmpMapper: FnMut(
        &mut Function<NewRegType, FullBlock<NewIType, NewRegType>>,
        &HashMap<
            RcEquality<Rc<RefCell<FullBlock<IType, RegType>>>>,
            Rc<RefCell<FullBlock<NewIType, NewRegType>>>,
        >,
        JumpInstruction<RegType::RValue, FullBlock<IType, RegType>>,
    ) -> (
        JmpInstIter,
        JumpInstruction<NewRegType::RValue, FullBlock<NewIType, NewRegType>>,
    ),
    LValueMapper: FnMut(RegType) -> NewRegType,
    RValueMapper: FnMut(RegType::RValue) -> NewRegType::RValue,
>(
    func: Function<RegType, FullBlock<IType, RegType>>,
    mut map_inst: InstMapper,
    mut map_jump: JmpMapper,
    mut map_lvalues: LValueMapper,
    mut map_rvalues: RValueMapper,
) -> Function<NewRegType, FullBlock<NewIType, NewRegType>> {
    let mut block_lookup = HashMap::new();
    for block in func.blocks() {
        block_lookup.insert(
            block.into(),
            Rc::new(RefCell::new(FullBlock::<NewIType, NewRegType>::default())),
        );
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
