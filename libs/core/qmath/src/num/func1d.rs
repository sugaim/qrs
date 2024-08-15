// -----------------------------------------------------------------------------
// Func1d
// -----------------------------------------------------------------------------
pub trait Func1d<Arg> {
    type Output;
    type Error;

    fn eval(&self, arg: &Arg) -> Result<Self::Output, Self::Error>;
}

// -----------------------------------------------------------------------------
// DerX1d
// -----------------------------------------------------------------------------
pub trait DerX1d<Arg>: Func1d<Arg> {
    type DerX;

    fn der_x(&self, arg: &Arg) -> Result<Self::DerX, Self::Error>;

    #[inline]
    fn der_0_x(&self, arg: &Arg) -> Result<(Self::Output, Self::DerX), Self::Error> {
        Ok((self.eval(arg)?, self.der_x(arg)?))
    }
}

// -----------------------------------------------------------------------------
// DerXX1d
// -----------------------------------------------------------------------------
pub trait DerXX1d<Arg>: DerX1d<Arg> {
    type DerXX;

    fn der_xx(&self, arg: &Arg) -> Result<Self::DerXX, Self::Error>;

    #[inline]
    #[allow(clippy::type_complexity)]
    fn der_0_x_xx(
        &self,
        arg: &Arg,
    ) -> Result<(Self::Output, Self::DerX, Self::DerXX), Self::Error> {
        let (val, der_x) = self.der_0_x(arg)?;
        Ok((val, der_x, self.der_xx(arg)?))
    }
}

// -----------------------------------------------------------------------------
// Integrable1d
// -----------------------------------------------------------------------------
pub trait Integrable1d<Arg>: Func1d<Arg> {
    type Integrated;

    fn integrate(&self, from: &Arg, to: &Arg) -> Result<Self::Integrated, Self::Error>;
}
