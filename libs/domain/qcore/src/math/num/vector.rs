use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::Zero;

use super::Arithmetic;

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
