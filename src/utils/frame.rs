use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

struct InnerFrame<K, V> {
    symbol_table: HashMap<K, V>,
    parent: Option<Frame<K, V>>,
}

pub struct Frame<K, V>(Rc<RefCell<InnerFrame<K, V>>>);

impl<'a, K: Hash + Eq, V: Copy> Frame<K, V> {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerFrame {
            symbol_table: HashMap::new(),
            parent: None,
        })))
    }

    pub fn new_child(&self) -> Self {
        Self(Rc::new(RefCell::new(InnerFrame {
            symbol_table: HashMap::new(),
            parent: Some((*self).clone()),
        })))
    }

    pub fn lookup<Q: Borrow<K>>(&self, name: &Q) -> Option<V> {
        (*self.0)
            .borrow()
            .symbol_table
            .get(name.borrow())
            .copied()
            .or_else(|| {
                (*self.0)
                    .borrow()
                    .parent
                    .as_ref()
                    .and_then(|p| p.lookup(name))
            })
    }

    pub fn assoc(&mut self, name: K, reg: V) {
        (*self.0).borrow_mut().symbol_table.insert(name, reg);
    }
}

impl<K, V> Clone for Frame<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
