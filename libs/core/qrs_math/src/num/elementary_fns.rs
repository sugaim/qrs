/// Trait to generalize the exponential function.
pub trait Exp: Sized {
    type Output: Into<Self>;
    fn exp(self) -> Self::Output;
}

impl Exp for f64 {
    type Output = Self;
    fn exp(self) -> Self::Output {
        f64::exp(self)
    }
}

impl Exp for f32 {
    type Output = Self;
    fn exp(self) -> Self::Output {
        f32::exp(self)
    }
}

/// Trait to generalize logarithm function.
pub trait Log: Sized {
    type Output: Into<Self>;
    fn log(self) -> Self::Output;
}

impl Log for f64 {
    type Output = Self;
    fn log(self) -> Self::Output {
        f64::ln(self)
    }
}

impl Log for f32 {
    type Output = Self;
    fn log(self) -> Self::Output {
        f32::ln(self)
    }
}
