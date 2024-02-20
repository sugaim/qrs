use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::{One, Zero};

use super::{Exp, Log, Vector};

/// Trait for arithmetic operations.
/// Intentionally declared loosely, such as no `Copy` requirement.
///
/// # Example
/// ```
/// use qrs_core::num::Arithmetic;
/// use static_assertions::assert_impl_all;
///
/// // integer types
/// assert_impl_all!(i8: Arithmetic);
/// assert_impl_all!(i16: Arithmetic);
/// assert_impl_all!(i32: Arithmetic);
/// assert_impl_all!(i64: Arithmetic);
/// assert_impl_all!(i128: Arithmetic);
/// assert_impl_all!(isize: Arithmetic);
///
/// // floating-point types
/// assert_impl_all!(f32: Arithmetic);
/// assert_impl_all!(f64: Arithmetic);
/// ```
///
pub trait Arithmetic:
    Clone
    + Zero
    + One
    + Neg<Output = Self>
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> AddAssign<&'a Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> MulAssign<&'a Self>
    + for<'a> Div<&'a Self, Output = Self>
    + for<'a> DivAssign<&'a Self>
{
}

impl<T> Arithmetic for T where
    T: Clone
        + Zero
        + One
        + Neg<Output = Self>
        + for<'a> Add<&'a Self, Output = Self>
        + for<'a> AddAssign<&'a Self>
        + for<'a> Sub<&'a Self, Output = Self>
        + for<'a> SubAssign<&'a Self>
        + for<'a> Mul<&'a Self, Output = Self>
        + for<'a> MulAssign<&'a Self>
        + for<'a> Div<&'a Self, Output = Self>
        + for<'a> DivAssign<&'a Self>
{
}

/// Some numeric types are based on some floating points.
/// This trait provides a way to access the base floating point type.
pub trait FloatBased {
    type BaseFloat: num::Float + Arithmetic;

    fn nearest_base_float_of(v: f64) -> Self::BaseFloat {
        <Self::BaseFloat as num::NumCast>::from(v).expect("Should calculate nearest value")
    }
}

impl FloatBased for f32 {
    type BaseFloat = f32;

    fn nearest_base_float_of(v: f64) -> f32 {
        v as f32
    }
}

impl FloatBased for f64 {
    type BaseFloat = f64;
}

/// Trait for scalar types.
/// This trait requires fundamental functions in addition to arithmetic operations.
///
/// # Example
/// ```
/// use qrs_core::num::Scalar;
/// use static_assertions::assert_impl_all;
///
/// assert_impl_all!(f32: Scalar);
/// assert_impl_all!(f64: Scalar);
/// ```
pub trait Scalar:
    Arithmetic
    + FloatBased
    + Vector<Self::BaseFloat>
    + From<Self::BaseFloat>
    + Exp<Output = Self>
    + Log<Output = Self>
{
    fn nearest_value_of(v: f64) -> Self {
        Self::from(<Self as FloatBased>::nearest_base_float_of(v))
    }
}

impl<T> Scalar for T where
    T: Arithmetic
        + FloatBased
        + Vector<Self::BaseFloat>
        + From<Self::BaseFloat>
        + Exp<Output = Self>
        + Log<Output = Self>
{
}

/// Trait for real numbers.
/// We consider a type `T` as a real number if it is a scalar on a 1-dim line.
/// Hence, this trait requires total ordering in addition to scalar requirements.
pub trait Real: Scalar + PartialEq + PartialOrd {}

impl<T> Real for T where T: Scalar + PartialEq + PartialOrd {}
