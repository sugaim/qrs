use crate::math::func1d::Func1d;

/// Trait for 1-dimensional interpolation.
pub trait Interp1d {
    type Grid: PartialOrd;
    type Value;

    /// Get the knots of the interpolation.
    ///
    /// Implementations must guarantee that the following conditions are satisfied:
    /// - `knots.0.len() == knots.1.len()`
    /// - `knots.0` is sorted in ascending order
    /// - `knots.0` has no duplicated elements
    fn knots(&self) -> (&[Self::Grid], &[Self::Value]);

    /// Interpolate the value at the given point.
    fn interp(&self, x: &Self::Grid) -> Self::Value;
}

impl<F: Interp1d> Func1d<F::Grid> for F {
    type Output = F::Value;

    fn eval(&self, x: &F::Grid) -> F::Value {
        self.interp(x)
    }
}

pub trait Interp1dBuilder<G, V> {
    type Output: Interp1d<Grid = G, Value = V>;
    type Error;

    fn build(self, grids: Vec<G>, values: Vec<V>) -> Result<Self::Output, Self::Error>;
}

pub trait DestructibleInterp1d: Interp1d {
    type Builer: Interp1dBuilder<Self::Grid, Self::Value, Output = Self>;

    fn destruct(self) -> (Self::Builer, Vec<Self::Grid>, Vec<Self::Value>);
}
