use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

// https://stackoverflow.com/questions/33847537/how-do-i-make-a-pointer-hashable

pub trait RcDereferencable {
    type Contained;
    fn as_ptr(&self) -> *const Self::Contained;
}

impl<T> RcDereferencable for Rc<T> {
    type Contained = T;
    fn as_ptr(&self) -> *const T {
        Rc::as_ptr(self)
    }
}

impl<T> RcDereferencable for Weak<T> {
    type Contained = T;
    fn as_ptr(&self) -> *const T {
        Weak::as_ptr(self)
    }
}
#[derive(Debug)]
pub struct RcEquality<T>(pub T, *const T::Contained)
where
    T: RcDereferencable;

impl<T: RcDereferencable> RcEquality<T> {
    pub fn get_ref(&self) -> &T {
        &self.0
    }
}

impl<T: RcDereferencable> Hash for RcEquality<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (self.1).hash(state)
    }
}

impl<T: RcDereferencable> PartialEq<RcEquality<T>> for RcEquality<T> {
    fn eq(&self, other: &RcEquality<T>) -> bool {
        std::ptr::eq(self.1, other.1)
    }
}

impl<T: RcDereferencable> Eq for RcEquality<T> {}

impl<T: RcDereferencable> Borrow<*const T::Contained> for RcEquality<T> {
    fn borrow(&self) -> &*const T::Contained {
        &self.1
    }
}

impl<T: RcDereferencable> From<T> for RcEquality<T> {
    fn from(x: T) -> Self {
        let ptr = x.as_ptr();
        RcEquality(x, ptr)
    }
}

pub trait RcEqualityKey<T: RcDereferencable> {
    fn as_key(&self) -> *const T::Contained;
}

impl<T: RcDereferencable> RcEqualityKey<T> for T {
    fn as_key(&self) -> *const T::Contained {
        self.as_ptr()
    }
}
