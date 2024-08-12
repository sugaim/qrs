use qcollections::flat_map::FlatMap;

use crate::num::Func1d;

// -----------------------------------------------------------------------------
// Interp1d
// -----------------------------------------------------------------------------
pub trait Interp1d {
    type X;
    type Output;

    fn interp(&self, x: &Self::X) -> Self::Output;

    fn interpolatee(&self) -> &FlatMap<Self::X, Self::Output>;
}

impl<I: Interp1d> Func1d<I::X> for I {
    type Output = I::Output;

    #[inline]
    fn eval(&self, x: &I::X) -> Self::Output {
        self.interp(x)
    }
}

// -----------------------------------------------------------------------------
// Interp1dBuilder
// RebuildableInterp1d
// -----------------------------------------------------------------------------
pub trait Interp1dBuilder<X, V> {
    type Output: Interp1d<X = X, Output = V>;

    fn build(self, data: FlatMap<X, V>) -> anyhow::Result<Self::Output>;
}

pub trait RebuildableInterp1d: Interp1d {
    type Builder: Interp1dBuilder<Self::X, Self::Output, Output = Self>;

    fn destruct(self) -> (Self::Builder, FlatMap<Self::X, Self::Output>);
}
