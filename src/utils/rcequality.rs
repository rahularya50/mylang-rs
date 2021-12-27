use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// https://stackoverflow.com/questions/33847537/how-do-i-make-a-pointer-hashable

#[derive(Debug)]
pub struct RcEquality<T>(pub Rc<T>, *const T);

impl<T> RcEquality<T> {
    pub fn get_ref(&self) -> &Rc<T> {
        &self.0
    }
}

impl<T> Hash for RcEquality<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (self.1).hash(state)
    }
}

impl<T> PartialEq<RcEquality<T>> for RcEquality<T> {
    fn eq(&self, other: &RcEquality<T>) -> bool {
        std::ptr::eq(self.1, other.1)
    }
}

impl<T> Eq for RcEquality<T> {}

impl<T> Borrow<*const T> for RcEquality<T> {
    fn borrow(&self) -> &*const T {
        &self.1
    }
}

impl<T> From<Rc<T>> for RcEquality<T> {
    fn from(x: Rc<T>) -> Self {
        let ptr = Rc::as_ptr(&x);
        RcEquality(x, ptr)
    }
}

pub trait RcEqualityKey<T> {
    fn as_key(&self) -> *const T;
}

impl<T> RcEqualityKey<T> for Rc<T> {
    fn as_key(&self) -> *const T {
        Rc::as_ptr(self)
    }
}
