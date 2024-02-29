use std::sync::{Arc, Mutex};

use crate::func1d::Func1d;

// -----------------------------------------------------------------------------
// Interp1d
//

/// Trait for 1-dimensional interpolation.
///
/// # Example
/// ```
/// use qrs_math::interp1d::Interp1d;
/// use qrs_math::interp1d::Lerp1d;
///
/// let grids = vec![0.0, 1.0, 2.0];
/// let values = vec![0.0, 1.0, 0.0];
///
/// let interp = Lerp1d::new(grids, values).unwrap();
///
/// // knots
/// let (knots, values) = interp.knots();
/// assert_eq!(knots, &[0.0, 1.0, 2.0]);
/// assert_eq!(values, &[0.0, 1.0, 0.0]);
///
/// // interpolation
/// assert_eq!(interp.interp(&-0.5), -0.5);
/// assert_eq!(interp.interp(&0.5), 0.5);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// ````
pub trait Interp1d {
    type Grid: PartialOrd;
    type Value;

    /// Interpolate the value at the given point.
    fn interp(&self, x: &Self::Grid) -> Self::Value;
}

impl<F: Interp1d> Func1d<F::Grid> for F {
    type Output = F::Value;

    #[inline]
    fn eval(&self, x: &F::Grid) -> F::Value {
        self.interp(x)
    }
}

impl<F: Interp1d> Interp1d for Arc<F> {
    type Grid = F::Grid;
    type Value = F::Value;

    #[inline]
    fn interp(&self, x: &Self::Grid) -> Self::Value {
        self.as_ref().interp(x)
    }
}

impl<F: Interp1d> Interp1d for Arc<Mutex<F>> {
    type Grid = F::Grid;
    type Value = F::Value;

    #[inline]
    fn interp(&self, x: &Self::Grid) -> Self::Value {
        self.lock().unwrap().interp(x)
    }
}

// -----------------------------------------------------------------------------
// Interp1dBuilder
//

/// Trait for building 1-dimensional interpolation.
///
/// # Example
/// ```
/// use qrs_math::interp1d::Interp1d;
/// use qrs_math::interp1d::Interp1dBuilder;
/// use qrs_math::interp1d::Lerp1dBuilder;;
///
/// let grids = vec![0.0f64, 1.0f64, 2.0f64];
/// let values = vec![0.0f64, 1.0f64, 0.0f64];
///
/// let interp = Lerp1dBuilder.build(grids, values).unwrap();
///
/// assert_eq!(interp.interp(&-0.5), -0.5);
/// assert_eq!(interp.interp(&0.5), 0.5);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// ```
pub trait Interp1dBuilder<G, V> {
    type Output: Interp1d<Grid = G, Value = V>;
    type Err;

    fn build(self, grids: Vec<G>, values: Vec<V>) -> Result<Self::Output, Self::Err>;
}

// -----------------------------------------------------------------------------
// DestructibleInterp1d
//

/// Trait for destructible 1-dimensional interpolation.
///
/// The destruction generates the builder, grids, and values.
/// Implementations must ensure that the output of generated builder with the grids and values
/// is the same as the original interpolation.
/// This property maybe useful when we want to modify knots of the interpolation safely.
///
/// # Example
/// ```
/// use qrs_math::interp1d::Interp1d;
/// use qrs_math::interp1d::Interp1dBuilder;
/// use qrs_math::interp1d::DestructibleInterp1d;
/// use qrs_math::interp1d::Lerp1d;
///
/// let grids = vec![0.0f64, 1.0f64, 2.0f64];
/// let values = vec![0.0f64, 1.0f64, 0.0f64];
///
/// let interp = Lerp1d::new(grids, values).unwrap();
/// let orig_interp = interp.clone();
///
/// // destruct and rebuild
/// let (builder, grids, values) = orig_interp.destruct();
/// let rebuilt_interp = builder.build(grids, values).unwrap();
///
/// assert_eq!(rebuilt_interp.interp(&-0.5), interp.interp(&-0.5));
/// assert_eq!(rebuilt_interp.interp(&0.5), interp.interp(&0.5));
/// assert_eq!(rebuilt_interp.interp(&1.0), interp.interp(&1.0));
/// ```
pub trait DestructibleInterp1d: Interp1d {
    type Builer: Interp1dBuilder<Self::Grid, Self::Value, Output = Self>;

    fn destruct(self) -> (Self::Builer, Vec<Self::Grid>, Vec<Self::Value>);
}
