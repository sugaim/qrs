use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use num::{One, Zero};

use super::{Erf, Exp, Log, Powi, Sqrt};

// -----------------------------------------------------------------------------
// FloatBased
//
/// Some numeric types are based on some floating points.
/// This trait provides a way to access the base floating point type.
pub trait FloatBased {
    type BaseFloat: num::Float + Arithmetic;

    #[inline]
    fn nearest_base_float_of_f64(v: f64) -> Self::BaseFloat {
        <Self::BaseFloat as num::NumCast>::from(v)
            .or_else(|| <Self::BaseFloat as num::NumCast>::from(v as f32))
            .expect("Should calculate nearest value")
    }
}

impl FloatBased for f32 {
    type BaseFloat = f32;

    #[inline]
    fn nearest_base_float_of_f64(v: f64) -> f32 {
        v as f32
    }
}

impl FloatBased for f64 {
    type BaseFloat = f64;

    #[inline]
    fn nearest_base_float_of_f64(v: f64) -> Self::BaseFloat {
        v
    }
}

impl FloatBased for ordered_float::OrderedFloat<f32> {
    type BaseFloat = f32;
}

impl FloatBased for ordered_float::OrderedFloat<f64> {
    type BaseFloat = f64;
}

// -----------------------------------------------------------------------------
// Arithmetic
// Vector
// -----------------------------------------------------------------------------
/// Trait for arithmetic operations.
/// Intentionally declared loosely, such as no `Copy` requirement.
///
/// # Example
/// ```
/// use qmath::num::Arithmetic;
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

/// Trait for (algebraic) vector like types.
/// This trait requires closed add/sub and scalar multiplication/division.
pub trait Vector<K: Arithmetic>:
    Clone
    + Zero
    + Neg<Output = Self>
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> AddAssign<&'a Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> Mul<&'a K, Output = Self>
    + for<'a> MulAssign<&'a K>
    + for<'a> Div<&'a K, Output = Self>
    + for<'a> DivAssign<&'a K>
{
}

impl<T, K> Vector<K> for T
where
    T: Clone
        + Zero
        + Neg<Output = T>
        + for<'a> Add<&'a T, Output = T>
        + for<'a> AddAssign<&'a T>
        + for<'a> Sub<&'a T, Output = T>
        + for<'a> SubAssign<&'a T>
        + for<'a> Mul<&'a K, Output = T>
        + for<'a> MulAssign<&'a K>
        + for<'a> Div<&'a K, Output = T>
        + for<'a> DivAssign<&'a K>,
    K: Arithmetic,
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

// -----------------------------------------------------------------------------
// Scalar
// Real
// -----------------------------------------------------------------------------
/// Trait for scalar types.
/// This trait requires fundamental functions in addition to arithmetic operations.
///
/// # Example
/// ```
/// use qmath::num::Scalar;
/// use static_assertions::assert_impl_all;
///
/// assert_impl_all!(f32: Scalar);
/// assert_impl_all!(f64: Scalar);
/// ```
pub trait Scalar:
    Arithmetic
    + FloatBased
    + PartialEq
    + Vector<Self::BaseFloat>
    + From<Self::BaseFloat>
    + Sqrt<Output = Self>
    + Powi<Output = Self>
    + Exp<Output = Self>
    + Log<Output = Self>
{
    #[inline]
    fn nearest_value_of_f64(v: f64) -> Self {
        Self::from(<Self as FloatBased>::nearest_base_float_of_f64(v))
    }
}

impl<T> Scalar for T where
    T: Arithmetic
        + FloatBased
        + PartialEq
        + Vector<Self::BaseFloat>
        + From<Self::BaseFloat>
        + Powi<Output = Self>
        + Sqrt<Output = Self>
        + Exp<Output = Self>
        + Log<Output = Self>
{
}

/// Trait for real numbers.
/// We consider a type `T` as a real number if it is a scalar on a 1-dim line.
/// Hence, this trait requires total ordering in addition to scalar requirements.
pub trait Real: Scalar + PartialOrd + Erf<Output = Self> + Display {}

impl<T> Real for T where T: Scalar + PartialOrd + Erf<Output = Self> + Display {}
