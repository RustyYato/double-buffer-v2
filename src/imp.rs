use crate::{
    base::Inner,
    traits::{RawDoubleBuffer, RawParts, Strategy, StrongBuffer, WeakBuffer},
};

unsafe impl<'a, S: Strategy, R: RawDoubleBuffer + ?Sized> RawParts for &'a mut Inner<R, S> {
    type Strategy = S;
    type Raw = R;

    type Strong = &'a Inner<R, S>;
    type Weak = &'a Inner<R, S>;

    fn raw_parts(self) -> (Self::Strong, Self::Weak) { (self, self) }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> StrongBuffer for &Inner<R, S> {
    type Strategy = S;
    type Raw = R;
    type Weak = Self;

    fn downgrade(&self) -> Self::Weak { self }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer + ?Sized> WeakBuffer for &Inner<R, S> {
    type Strategy = S;
    type Raw = R;
    type Strong = Self;
    type UpgradeError = core::convert::Infallible;

    fn is_dangling(&self) -> bool { false }

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError> { Ok(*self) }
}
