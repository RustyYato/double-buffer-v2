use crate::{
    thin,
    traits::{RawParts, StrongBuffer, WeakBuffer},
};
use std::{boxed::Box, rc, sync};

use crate::{
    base::Inner,
    traits::{RawDoubleBuffer, Strategy},
};

#[derive(Debug)]
pub struct UpgradeError;

unsafe impl<'a, S: Strategy, R: RawDoubleBuffer + ?Sized> RawParts for sync::Arc<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;

    type Strong = Self;
    type Weak = sync::Weak<Inner<R, S>>;

    fn raw_parts(mut self) -> (Self::Strong, Self::Weak) {
        sync::Arc::get_mut(&mut self).expect("Arc mut be unique");
        let weak = sync::Arc::downgrade(&self);
        (self, weak)
    }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> StrongBuffer for sync::Arc<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Weak = sync::Weak<Inner<R, S>>;

    fn downgrade(&self) -> Self::Weak { sync::Arc::downgrade(self) }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> WeakBuffer for sync::Weak<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Strong = sync::Arc<Inner<R, S>>;
    type UpgradeError = UpgradeError;

    fn is_dangling(&self) -> bool { sync::Weak::strong_count(self) == 0 }

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError> { self.upgrade().ok_or(UpgradeError) }
}

unsafe impl<'a, S: Strategy, R: RawDoubleBuffer + ?Sized> RawParts for rc::Rc<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;

    type Strong = Self;
    type Weak = rc::Weak<Inner<R, S>>;

    fn raw_parts(mut self) -> (Self::Strong, Self::Weak) {
        rc::Rc::get_mut(&mut self).expect("Rc mut be unique");
        let weak = rc::Rc::downgrade(&self);
        (self, weak)
    }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> StrongBuffer for rc::Rc<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Weak = rc::Weak<Inner<R, S>>;

    fn downgrade(&self) -> Self::Weak { rc::Rc::downgrade(self) }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> WeakBuffer for rc::Weak<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Strong = rc::Rc<Inner<R, S>>;
    type UpgradeError = UpgradeError;

    fn is_dangling(&self) -> bool { rc::Weak::strong_count(self) == 0 }

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError> { self.upgrade().ok_or(UpgradeError) }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> RawParts for Box<thin::ThinInner<Inner<R, S>>> {
    type Strategy = S;
    type Raw = R;

    type Strong = thin::Thin<Inner<R, S>>;
    type Weak = thin::Thin<Inner<R, S>>;

    fn raw_parts(self) -> (Self::Strong, Self::Weak) {
        let thin = thin::Thin::from(self);
        (thin.clone(), thin)
    }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> StrongBuffer for thin::Thin<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Weak = Self;

    fn downgrade(&self) -> Self::Weak { self.clone() }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> WeakBuffer for thin::Thin<Inner<R, S>> {
    type Strategy = S;
    type Raw = R;
    type Strong = Self;
    type UpgradeError = core::convert::Infallible;

    fn is_dangling(&self) -> bool { false }

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError> { Ok(self.clone()) }
}
