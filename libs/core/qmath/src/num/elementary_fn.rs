// -----------------------------------------------------------------------------
// Sqrt
// -----------------------------------------------------------------------------
pub trait Sqrt: Sized {
    type Output: Into<Self>;

    fn sqrt(self) -> Self::Output;
}

impl Sqrt for f64 {
    type Output = Self;

    #[inline]
    fn sqrt(self) -> Self::Output {
        f64::sqrt(self)
    }
}

impl Sqrt for f32 {
    type Output = Self;

    #[inline]
    fn sqrt(self) -> Self::Output {
        f32::sqrt(self)
    }
}

impl<T: Sqrt<Output = T>> Sqrt for ordered_float::OrderedFloat<T> {
    type Output = ordered_float::OrderedFloat<T::Output>;

    #[inline]
    fn sqrt(self) -> Self::Output {
        ordered_float::OrderedFloat(self.0.sqrt())
    }
}

// -----------------------------------------------------------------------------
// Powi
// -----------------------------------------------------------------------------
/// Trait to generalize integer power function interface.
pub trait Powi: Sized {
    type Output: Into<Self>;

    fn powi(self, n: i32) -> Self::Output;
}

impl Powi for f64 {
    type Output = Self;

    #[inline]
    fn powi(self, n: i32) -> Self::Output {
        f64::powi(self, n)
    }
}

impl Powi for f32 {
    type Output = Self;

    #[inline]
    fn powi(self, n: i32) -> Self::Output {
        f32::powi(self, n)
    }
}

impl<T: Powi<Output = T>> Powi for ordered_float::OrderedFloat<T> {
    type Output = ordered_float::OrderedFloat<T::Output>;

    #[inline]
    fn powi(self, n: i32) -> Self::Output {
        ordered_float::OrderedFloat(self.0.powi(n))
    }
}

// -----------------------------------------------------------------------------
// Exp
// -----------------------------------------------------------------------------
/// Trait to provide the exponential function interface.
pub trait Exp: Sized {
    type Output: Into<Self>;

    fn exp(self) -> Self::Output;
}

impl Exp for f64 {
    type Output = Self;

    #[inline]
    fn exp(self) -> Self::Output {
        f64::exp(self)
    }
}

impl Exp for f32 {
    type Output = Self;

    #[inline]
    fn exp(self) -> Self::Output {
        f32::exp(self)
    }
}

impl<T: Exp<Output = T>> Exp for ordered_float::OrderedFloat<T> {
    type Output = ordered_float::OrderedFloat<T::Output>;

    #[inline]
    fn exp(self) -> Self::Output {
        ordered_float::OrderedFloat(self.0.exp())
    }
}

// -----------------------------------------------------------------------------
// Log
// -----------------------------------------------------------------------------
/// Trait to generalize logarithm (in natural base) function interface.
pub trait Log: Sized {
    type Output: Into<Self>;

    fn log(self) -> Self::Output;
}

impl Log for f64 {
    type Output = Self;

    #[inline]
    fn log(self) -> Self::Output {
        f64::ln(self)
    }
}

impl Log for f32 {
    type Output = Self;

    #[inline]
    fn log(self) -> Self::Output {
        f32::ln(self)
    }
}

impl<T: Log<Output = T>> Log for ordered_float::OrderedFloat<T> {
    type Output = ordered_float::OrderedFloat<T::Output>;

    #[inline]
    fn log(self) -> Self::Output {
        ordered_float::OrderedFloat(self.0.log())
    }
}

// -----------------------------------------------------------------------------
// Erf
// -----------------------------------------------------------------------------
/// Trait to generalize error function interface.
pub trait Erf: Sized {
    type Output: Into<Self>;

    fn erf(self) -> Self::Output;
}

impl Erf for f64 {
    type Output = Self;

    #[inline]
    fn erf(self) -> Self::Output {
        libm::erf(self)
    }
}

impl Erf for f32 {
    type Output = Self;

    #[inline]
    fn erf(self) -> Self::Output {
        libm::erff(self)
    }
}

impl<T: Erf<Output = T>> Erf for ordered_float::OrderedFloat<T> {
    type Output = ordered_float::OrderedFloat<T::Output>;

    #[inline]
    fn erf(self) -> Self::Output {
        ordered_float::OrderedFloat(self.0.erf())
    }
}
