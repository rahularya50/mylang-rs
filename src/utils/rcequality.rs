use std::hash::{Hash, Hasher};
use std::rc::Rc;

// https://stackoverflow.com/questions/33847537/how-do-i-make-a-pointer-hashable

#[derive(Debug)]
pub struct RcEquality<T>(T);

impl<T> Hash for RcEquality<Rc<T>> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (Rc::as_ptr(&self.0)).hash(state)
    }
}

impl<T> PartialEq<RcEquality<Rc<T>>> for RcEquality<Rc<T>> {
    fn eq(&self, other: &RcEquality<Rc<T>>) -> bool {
        std::ptr::eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl<T> Eq for RcEquality<Rc<T>> {}

impl<T> From<Rc<T>> for RcEquality<Rc<T>> {
    fn from(x: Rc<T>) -> Self {
        RcEquality(x)
    }
}
