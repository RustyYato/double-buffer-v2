use crate::{
    base::Inner,
    traits::{RawDoubleBuffer, RawParts, Strategy, StrongBuffer, WeakBuffer},
};

unsafe impl<'a, S: Strategy, R: RawDoubleBuffer> RawParts for &'a mut Inner<S, R> {
    type Strategy = S;
    type Raw = R;

    type Strong = &'a Inner<S, R>;
    type Weak = &'a Inner<S, R>;

    fn raw_parts(self) -> (Self::Strong, Self::Weak) { (self, self) }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer> StrongBuffer for &Inner<S, R> {
    type Strategy = S;
    type Raw = R;
    type Weak = Self;

    fn downgrade(&self) -> Self::Weak { self }
}

unsafe impl<S: Strategy, R: RawDoubleBuffer> WeakBuffer for &Inner<S, R> {
    type Strategy = S;
    type Raw = R;
    type Strong = Self;
    type UpgradeError = core::convert::Infallible;

    fn is_dangling(&self) -> bool { false }

    fn upgrade(&self) -> Result<Self::Strong, Self::UpgradeError> { Ok(*self) }
}
