use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Frame<'a, K, V> {
    symbol_table: HashMap<K, V>,
    parent: Option<&'a Frame<'a, K, V>>,
}

impl<'a, K: Hash + Eq, V: Copy> Frame<'a, K, V> {
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
            parent: None,
        }
    }

    pub fn new_child(&'a self) -> Frame<K, V> {
        Self {
            symbol_table: HashMap::new(),
            parent: Some(self),
        }
    }

    pub fn lookup<Q: Borrow<K>>(&self, name: &Q) -> Option<V> {
        self.symbol_table
            .get(name.borrow())
            .copied()
            .or_else(|| self.parent.and_then(|p| p.lookup(name)))
    }

    pub fn assoc(&mut self, name: K, reg: V) {
        self.symbol_table.insert(name, reg);
    }
}
