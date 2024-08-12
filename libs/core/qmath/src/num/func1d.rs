// -----------------------------------------------------------------------------
// Func1d
// -----------------------------------------------------------------------------
pub trait Func1d<Arg> {
    type Output;

    fn eval(&self, arg: &Arg) -> Self::Output;
}

// -----------------------------------------------------------------------------
// DerX1d
// -----------------------------------------------------------------------------
pub trait DerX1d<Arg>: Func1d<Arg> {
    type DerX;

    fn der_x(&self, arg: &Arg) -> Self::DerX;

    #[inline]
    fn der_0_x(&self, arg: &Arg) -> (Self::Output, Self::DerX) {
        (self.eval(arg), self.der_x(arg))
    }
}

// -----------------------------------------------------------------------------
// DerXX1d
// -----------------------------------------------------------------------------
pub trait DerXX1d<Arg>: DerX1d<Arg> {
    type DerXX;

    fn der_xx(&self, arg: &Arg) -> Self::DerXX;

    #[inline]
    fn der_0_x_xx(&self, arg: &Arg) -> (Self::Output, Self::DerX, Self::DerXX) {
        let (val, der_x) = self.der_0_x(arg);
        (val, der_x, self.der_xx(arg))
    }
}

// -----------------------------------------------------------------------------
// Integrable1d
// -----------------------------------------------------------------------------
pub trait Integrable1d<Arg>: Func1d<Arg> {
    type Integrated;

    fn integrate(&self, from: &Arg, to: &Arg) -> Self::Integrated;
}
