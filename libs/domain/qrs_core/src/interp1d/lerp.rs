use std::ops::{Add, Div, Mul, Neg, Sub};

use num::{One, Zero};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    func1d::{Func1dDer1, Func1dDer2, Func1dIntegrable},
    num::{FloatBased, RelPos, Vector},
};

use super::{DestructibleInterp1d, Interp1d, Interp1dBuilder, _knots::Knots};

// -----------------------------------------------------------------------------
// Lerp1d
//

/// 1-dimensional linear interpolation.
///
/// # Example
/// ```
/// use qrs_core::interp1d::Interp1d;
///
/// let grids = vec![0.0, 1.0, 2.0];
/// let values = vec![0.0, 1.0, 0.0];
///
/// let interp = qrs_core::interp1d::Lerp1d::new(grids, values).unwrap();
///
/// assert_eq!(interp.interp(&-0.5), -0.5);
/// assert_eq!(interp.interp(&0.5), 0.5);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "1-dimensional linear interpolation")]
pub struct Lerp1d<G, V> {
    #[serde(bound(
        serialize = "G: Serialize + PartialOrd, V: Serialize",
        deserialize = "G: Deserialize<'de> + PartialOrd, V: Deserialize<'de>"
    ))]
    knots: Knots<G, V>,
}

//
// construction
//
impl<G: PartialOrd, V> Lerp1d<G, V> {
    /// Create a new `Lerp1d` interpolation.
    ///
    /// # Errors
    /// - If the length of `gs` is less than 2.
    /// - If the length of `gs` and `vs` are not equal.
    /// - If `gs` is not sorted in ascending order.
    #[inline]
    pub fn new(gs: Vec<G>, vs: Vec<V>) -> Result<Self, anyhow::Error> {
        Knots::new(gs, vs).map(|knots| Self { knots })
    }
}

//
// methods
//
impl<G: RelPos, V: Vector<G::Output>> Interp1d for Lerp1d<G, V> {
    type Grid = G;
    type Value = V;

    #[inline]
    fn knots(&self) -> (&[G], &[V]) {
        (self.knots.grids(), self.knots.values())
    }

    fn interp(&self, x: &G) -> V {
        let idx = self.knots.interval_index_of(x);
        let (gl, vl) = self.knots.force_get(idx);
        let (gr, vr) = self.knots.force_get(idx + 1);

        // weights for left and right knots
        let wr = x.relpos_between(gl, gr);
        let wl = <G::Output as One>::one() - &wr;

        vl.clone() * &wl + vr.clone() * &wr
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>> Func1dDer1<G> for Lerp1d<G, V>
where
    G: Clone + Sub<G>,
    V: Div<<G as Sub>::Output>,
{
    type Der1 = <V as Div<<G as Sub>::Output>>::Output;

    fn der1(&self, x: &G) -> Self::Der1 {
        let idx = self.knots.interval_index_of(x);
        let (gl, vl) = self.knots.force_get(idx);
        let (gr, vr) = self.knots.force_get(idx + 1);

        let dv = vr.clone() - vl;
        let dg = gr.clone() - gl.clone();
        dv / dg
    }
    fn der01(&self, x: &G) -> (Self::Output, Self::Der1) {
        // to reduce search cost, we override the default implementation
        let idx = self.knots.interval_index_of(x);
        let (gl, vl) = self.knots.force_get(idx);
        let (gr, vr) = self.knots.force_get(idx + 1);

        // der0
        let wr = x.relpos_between(gl, gr);
        let wl = <<G as RelPos>::Output as One>::one() - &wr;
        let der0 = vl.clone() * &wl + vr.clone() * &wr;

        // der1
        let dv = vr.clone() - vl;
        let dg = gr.clone() - gl.clone();
        let der1 = dv / dg;

        (der0, der1)
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>> Func1dDer2<G> for Lerp1d<G, V>
where
    G: Clone + Sub<G>,
    V: Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Div<<G as Sub>::Output>,
    <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output: Zero,
{
    type Der2 = <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output;

    fn der2(&self, _: &G) -> Self::Der2 {
        Zero::zero()
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>, O> Func1dIntegrable<G> for Lerp1d<G, V>
where
    G: Clone + Sub<G>,
    V: Mul<<G as Sub>::Output, Output = O>,
    O: Add<Output = O> + Neg<Output = O>,
{
    type Integrated = <V as Mul<<G as Sub>::Output>>::Output;

    fn integrate(&self, from: &G, to: &G) -> Self::Integrated {
        if to < from {
            return -self.integrate(to, from);
        }
        let lidx = self.knots.interval_index_of(from);
        let ridx = self.knots.interval_index_of(to);

        let one = <<G as RelPos>::Output as One>::one();
        let half = <<G as RelPos>::Output as FloatBased>::nearest_base_float_of(0.5);

        if lidx == ridx {
            let (gl, vl) = self.knots.force_get(lidx);
            let (gr, vr) = self.knots.force_get(ridx + 1);
            let wf = from.relpos_between(gl, gr);
            let wt = to.relpos_between(gl, gr);
            let yf = vl.clone() * &(one.clone() - &wf) + vr.clone() * &wf;
            let yt = vl.clone() * &(one.clone() - &wt) + vr.clone() * &wt;
            let mid = (yf + &yt) * &half.into();
            return mid * (to.clone() - from.clone());
        }
        let left_contrib = {
            let (gl, vl) = self.knots.force_get(lidx);
            let (gr, vr) = self.knots.force_get(lidx + 1);
            let w = from.relpos_between(gl, gr);
            let y = vl.clone() * &(one.clone() - &w) + vr.clone() * &w;
            let mid = (y + vr) * &half.into();
            mid * (gr.clone() - from.clone())
        };
        let right_contrib = {
            let (gl, vl) = self.knots.force_get(ridx);
            let (gr, vr) = self.knots.force_get(ridx + 1);
            let w = to.relpos_between(gl, gr);
            let y = vl.clone() * &(one.clone() - &w) + vr.clone() * &w;
            let mid = (y + vl) * &half.into();
            mid * (to.clone() - gl.clone())
        };
        let mut res = left_contrib + right_contrib;
        for i in lidx + 1..ridx {
            let (gl, vl) = self.knots.force_get(i);
            let (gr, vr) = self.knots.force_get(i + 1);
            let mid = (vl.clone() + vr.clone()) * &half.into();
            res = res + mid * (gr.clone() - gl.clone());
        }
        res
    }
}

// -----------------------------------------------------------------------------
// Lerp1dBuilder
//
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Lerp1dBuilder;

//
// construction
//
impl Default for Lerp1dBuilder {
    fn default() -> Self {
        Self
    }
}

//
// methods
//
impl<G: RelPos, V: Vector<G::Output>> Interp1dBuilder<G, V> for Lerp1dBuilder {
    type Output = Lerp1d<G, V>;
    type Err = anyhow::Error;

    fn build(self, gs: Vec<G>, vs: Vec<V>) -> Result<Self::Output, anyhow::Error> {
        Lerp1d::new(gs, vs)
    }
}

impl<G: RelPos, V: Vector<G::Output>> DestructibleInterp1d for Lerp1d<G, V> {
    type Builer = Lerp1dBuilder;

    fn destruct(self) -> (Self::Builer, Vec<G>, Vec<V>) {
        let (gs, vs) = self.knots.destruct();
        (Lerp1dBuilder, gs, vs)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use crate::func1d::Func1d;

    use super::*;

    #[test]
    fn test_lerp1d_new() {
        // length mismatch
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 1.];

        let lerp = Lerp1d::new(grids, values);
        assert!(lerp.is_err());

        // not sorted
        let grids: Vec<f64> = vec![0., 2., 1.];
        let values: Vec<f64> = vec![0., 1., 2.];

        let lerp = Lerp1d::new(grids, values);
        assert!(lerp.is_err());

        // success
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 1., 2.];

        let lerp = Lerp1d::new(grids, values);
        assert!(lerp.is_ok());
    }

    #[test]
    fn test_lerp1d_serialize() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 1., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let serialized = serde_json::to_string(&lerp).unwrap();
        assert_eq!(serialized, r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,2.0]]}"#);
    }

    #[test]
    fn test_lerp1d_deserialize() {
        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,2.0]]}"#;
        let deserialized: Lerp1d<f64, f64> = serde_json::from_str(serialized).unwrap();

        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 1., 2.];
        let lerp = Lerp1d::new(grids, values).unwrap();
        assert_eq!(deserialized, lerp);
    }

    #[test]
    fn test_lerp1d_knots() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 5., 10.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let (grids, values) = lerp.knots();

        assert_eq!(grids, &[0., 1., 2.]);
        assert_eq!(values, &[0., 5., 10.]);
    }

    #[test]
    fn test_lerp1d_interp() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        assert_abs_diff_eq!(lerp.interp(&-1.), -3., epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&-0.5), -1.5, epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&0.), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&0.5), 1.5, epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&1.), 3., epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&1.5), 2.5, epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&2.), 2., epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&2.5), 1.5, epsilon = eps);
        assert_abs_diff_eq!(lerp.interp(&3.), 1., epsilon = eps);
    }

    #[test]
    fn test_lerp1d_der1() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        assert_abs_diff_eq!(lerp.der1(&-1.), 3., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&-0.5), 3., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&0.), 3., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&0.5), 3., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&1.), -1., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&1.5), -1., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&2.), -1., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&2.5), -1., epsilon = eps);
        assert_abs_diff_eq!(lerp.der1(&3.), -1., epsilon = eps);
    }

    #[test]
    fn test_lerp1d_der01() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        let x = -1.;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = -0.5;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 0.;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 0.5;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 1.;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 1.5;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 2.;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 2.5;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);

        let x = 3.;
        let (der0, der1) = lerp.der01(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
    }

    #[test]
    fn test_lerp1d_der2() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        assert_abs_diff_eq!(lerp.der2(&-1.), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&-0.5), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&0.), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&0.5), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&1.), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&1.5), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&2.), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&2.5), 0., epsilon = eps);
        assert_abs_diff_eq!(lerp.der2(&3.), 0., epsilon = eps);
    }

    #[test]
    fn test_lerp_der012() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        let x = -1.;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = -0.5;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 0.;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 0.5;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 1.;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 1.5;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 2.;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 2.5;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);

        let x = 3.;
        let (der0, der1, der2) = lerp.der012(&x);
        assert_abs_diff_eq!(der0, lerp.eval(&x), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&x), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&x), epsilon = eps);
    }

    #[test]
    fn test_integrate() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let eps = 1e-15;

        let from = -2.;
        let to = -1.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, -4.5, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = -2.;
        let to = 0.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, -6., epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 0.;
        let to = 0.5;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 0.375, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = -2.;
        let to = 0.5;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, -5.625, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 0.;
        let to = 1.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 1.5, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 1.5;
        let to = 2.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 1.125, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 2.;
        let to = 3.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 1.5, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 3.;
        let to = 4.;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 0.5, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);

        let from = 1.5;
        let to = 3.0;
        let integral = lerp.integrate(&from, &to);
        assert_abs_diff_eq!(integral, 2.625, epsilon = eps);
        assert_abs_diff_eq!(-integral, lerp.integrate(&to, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&from, &from), epsilon = eps);
        assert_abs_diff_eq!(0., lerp.integrate(&to, &to), epsilon = eps);
    }

    #[test]
    fn test_lerp1d_destruct() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1d::new(grids, values).unwrap();
        let (builder, grids, values) = lerp.destruct();

        assert_eq!(builder, Lerp1dBuilder);
        assert_eq!(grids, vec![0., 1., 2.]);
        assert_eq!(values, vec![0., 3., 2.]);
    }

    #[test]
    fn test_lerp1d_build() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];

        let lerp = Lerp1dBuilder.build(grids.clone(), values.clone()).unwrap();
        assert_eq!(lerp.knots().0, grids.as_slice());
        assert_eq!(lerp.knots().1, values.as_slice());

        let (builder, grids, values) = lerp.destruct();
        assert_eq!(builder, Lerp1dBuilder);
        assert_eq!(grids, vec![0., 1., 2.]);
        assert_eq!(values, vec![0., 3., 2.]);
    }
}
