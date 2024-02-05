/// Trait for 1-dimensional function.
pub trait Func1d<X> {
    type Output;

    fn eval(&self, x: &X) -> Self::Output;
}

/// Trait for 1-dimensional function with its first derivative.
pub trait Func1dDer1<X>: Func1d<X> {
    type Der1;

    fn der1(&self, x: &X) -> Self::Der1;

    fn der01(&self, x: &X) -> (Self::Output, Self::Der1) {
        (self.eval(x), self.der1(x))
    }
}

/// Trait for 1-dimensional function with its first and second derivatives.
pub trait Func1dDer2<X>: Func1dDer1<X> {
    type Der2;

    fn der2(&self, x: &X) -> Self::Der2;

    fn der012(&self, x: &X) -> (Self::Output, Self::Der1, Self::Der2) {
        let (der0, der1) = self.der01(x);
        (der0, der1, self.der2(x))
    }
}
