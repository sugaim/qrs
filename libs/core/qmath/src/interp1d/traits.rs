use qcollections::flat_map::FlatMap;

use crate::num::Func1d;

// -----------------------------------------------------------------------------
// Interp1d
// -----------------------------------------------------------------------------
pub trait Interp1d {
    type X;
    type Value;

    fn interp(&self, x: &Self::X) -> anyhow::Result<Self::Value>;

    fn interpolatee(&self) -> &FlatMap<Self::X, Self::Value>;
}

impl<I: Interp1d> Func1d<I::X> for I {
    type Output = I::Value;
    type Error = anyhow::Error;

    #[inline]
    fn eval(&self, arg: &I::X) -> Result<Self::Output, Self::Error> {
        self.interp(arg)
    }
}

// -----------------------------------------------------------------------------
// Interp1dBuilder
// RebuildableInterp1d
// -----------------------------------------------------------------------------
pub trait Interp1dBuilder<X, V> {
    type Output: Interp1d<X = X, Value = V>;

    fn build(self, data: FlatMap<X, V>) -> anyhow::Result<Self::Output>;
}

pub trait RebuildableInterp1d: Interp1d {
    type Builder: Interp1dBuilder<Self::X, Self::Value, Output = Self>;

    fn destruct(self) -> (Self::Builder, FlatMap<Self::X, Self::Value>);
}
