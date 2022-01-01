use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use super::rcequality::RcEqualityKey;

pub struct UnionFind<T> {
    lookup: HashMap<T, Rc<RefCell<UnionFindNode<T>>>>,
}

pub struct UnionFindNode<T> {
    pub value: T,
    parent: Option<Rc<RefCell<UnionFindNode<T>>>>,
    size: u16,
}

impl<T: Eq + Hash + Clone> UnionFind<T> {
    pub fn new() -> Self {
        UnionFind {
            lookup: HashMap::new(),
        }
    }

    pub fn find_root(&self, a: &T) -> Option<Rc<RefCell<UnionFindNode<T>>>> {
        let mut pos = self.lookup.get(a)?.clone();
        loop {
            let parent = pos.borrow().parent.clone();
            if let Some(parent) = parent {
                pos = parent;
            } else {
                break;
            }
        }
        Some(pos)
    }

    fn insert(&mut self, value: T) -> Rc<RefCell<UnionFindNode<T>>> {
        let out = Rc::new(RefCell::new(UnionFindNode {
            value: value.clone(),
            parent: None,
            size: 1,
        }));
        self.lookup.insert(value, out.clone());
        out
    }

    pub fn union(&mut self, a: T, b: T) {
        let a = self.find_root(&a).unwrap_or_else(|| self.insert(a));
        let b = self.find_root(&b).unwrap_or_else(|| self.insert(b));
        if a.as_key() == b.as_key() {
            return;
        }
        if a.borrow().size > b.borrow().size {
            self.link_nodes(a, b);
        } else {
            self.link_nodes(b, a);
        }
    }

    pub fn directed_union(&mut self, parent: T, child: T) {
        let parent = self
            .lookup
            .get(&parent)
            .cloned()
            .unwrap_or_else(|| self.insert(parent));
        let child = self
            .lookup
            .get(&child)
            .cloned()
            .unwrap_or_else(|| self.insert(child));
        self.link_nodes(parent, child);
    }

    fn link_nodes(
        &mut self,
        parent: Rc<RefCell<UnionFindNode<T>>>,
        child: Rc<RefCell<UnionFindNode<T>>>,
    ) {
        let mut parent_mut = parent.borrow_mut();
        let mut child_mut = child.borrow_mut();
        parent_mut.size += child_mut.size;
        drop(parent_mut);
        child_mut.parent = Some(parent);
    }
}
