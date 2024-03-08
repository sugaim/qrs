use std::ops::{Add, Div, Mul, Neg, Sub};

use qrs_collections::{MinSized, RequireMinSize, Series};

use crate::func1d::{Func1dDer1, Func1dDer2, Func1dIntegrable};
use crate::interp1d::{DestructibleInterp1d, Interp1d, Interp1dBuilder};
use crate::num::{FloatBased, One, RelPos, Vector, Zero};

// -----------------------------------------------------------------------------
// Lerp1d
//

/// 1-dimensional linear interpolation.
///
/// # Example
/// ```
/// use qrs_collections::{Series, RequireMinSize};
/// use qrs_math::interp1d::{Interp1d, Lerp1d};
///
/// let grids = vec![0.0, 1.0, 2.0];
/// let values = vec![0.0, 1.0, 0.0];
/// let knots = Series::new(grids, values).unwrap().require_min_size().unwrap();
///
/// let interp = Lerp1d::new(knots);
///
/// assert_eq!(interp.interp(&-0.5), -0.5);
/// assert_eq!(interp.interp(&0.5), 0.5);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    schemars(description = "1-dimensional linear interpolation")
)]
pub struct Lerp1d<G, V> {
    #[cfg_attr(
        feature = "serde",
        serde(bound(
            serialize = "G: serde::Serialize + PartialOrd, V: serde::Serialize",
            deserialize = "G: serde::Deserialize<'de> + PartialOrd, V: serde::Deserialize<'de>"
        ))
    )]
    knots: MinSized<Series<G, V>, 2>,
}

//
// construction
//
impl<G, V> Lerp1d<G, V> {
    /// Create a new `Lerp1d` interpolation.
    #[inline]
    pub fn new(knots: MinSized<Series<G, V>, 2>) -> Self {
        Self { knots }
    }
}

//
// methods
//
impl<G, V> Lerp1d<G, V> {
    #[inline]
    pub fn knots(&self) -> (&[G], &[V]) {
        (self.knots.grids(), self.knots.values())
    }
}

impl<G: RelPos, V: Vector<G::Output>> Interp1d for Lerp1d<G, V> {
    type Grid = G;
    type Value = V;

    fn interp(&self, x: &G) -> V {
        let idx = self.knots.interval_index_of(x).unwrap();
        let (gl, vl) = self.knots.get(idx).unwrap();
        let (gr, vr) = self.knots.get(idx + 1).unwrap();

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
        let idx = self.knots.interval_index_of(x).unwrap();
        let (gl, vl) = self.knots.get(idx).unwrap();
        let (gr, vr) = self.knots.get(idx + 1).unwrap();

        let dv = vr.clone() - vl;
        let dg = gr.clone() - gl.clone();
        dv / dg
    }
    fn der01(&self, x: &G) -> (Self::Output, Self::Der1) {
        // to reduce search cost, we override the default implementation
        let idx = self.knots.interval_index_of(x).unwrap();
        let (gl, vl) = self.knots.get(idx).unwrap();
        let (gr, vr) = self.knots.get(idx + 1).unwrap();

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
        let lidx = self.knots.interval_index_of(from).unwrap();
        let ridx = self.knots.interval_index_of(to).unwrap();

        let one = <<G as RelPos>::Output as One>::one();
        let half = <<G as RelPos>::Output as FloatBased>::nearest_base_float_of(0.5);

        if lidx == ridx {
            let (gl, vl) = self.knots.get(lidx).unwrap();
            let (gr, vr) = self.knots.get(ridx + 1).unwrap();
            let wf = from.relpos_between(gl, gr);
            let wt = to.relpos_between(gl, gr);
            let yf = vl.clone() * &(one.clone() - &wf) + vr.clone() * &wf;
            let yt = vl.clone() * &(one.clone() - &wt) + vr.clone() * &wt;
            let mid = (yf + &yt) * &half.into();
            return mid * (to.clone() - from.clone());
        }
        let left_contrib = {
            let (gl, vl) = self.knots.get(lidx).unwrap();
            let (gr, vr) = self.knots.get(lidx + 1).unwrap();
            let w = from.relpos_between(gl, gr);
            let y = vl.clone() * &(one.clone() - &w) + vr.clone() * &w;
            let mid = (y + vr) * &half.into();
            mid * (gr.clone() - from.clone())
        };
        let right_contrib = {
            let (gl, vl) = self.knots.get(ridx).unwrap();
            let (gr, vr) = self.knots.get(ridx + 1).unwrap();
            let w = to.relpos_between(gl, gr);
            let y = vl.clone() * &(one.clone() - &w) + vr.clone() * &w;
            let mid = (y + vl) * &half.into();
            mid * (to.clone() - gl.clone())
        };
        let mut res = left_contrib + right_contrib;
        for i in lidx + 1..ridx {
            let (gl, vl) = self.knots.get(i).unwrap();
            let (gr, vr) = self.knots.get(i + 1).unwrap();
            let mid = (vl.clone() + vr.clone()) * &half.into();
            res = res + mid * (gr.clone() - gl.clone());
        }
        res
    }
}

// -----------------------------------------------------------------------------
// Lerp1dBuilder
//
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    schemars(description = "1-dimensional linear interpolation builder")
)]
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

    fn build(self, knots: Series<G, V>) -> Result<Self::Output, anyhow::Error> {
        knots
            .require_min_size()
            .map_err(Into::into)
            .map(|knots| Lerp1d::new(knots))
    }
}

impl<G: RelPos, V: Vector<G::Output>> DestructibleInterp1d for Lerp1d<G, V> {
    type Builer = Lerp1dBuilder;

    fn destruct(self) -> (Self::Builer, Series<G, V>) {
        (Lerp1dBuilder, self.knots.into_inner())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use qrs_collections::RequireMinSize;
    use rstest::rstest;

    use crate::func1d::Func1d;

    use super::*;

    #[test]
    fn test_lerp1d_new() {
        //
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 1., 2.];
        let knots = Series::new(grids, values)
            .unwrap()
            .require_min_size()
            .unwrap();

        let lerp = Lerp1d::new(knots);

        assert_eq!(lerp.knots.grids(), &[0., 1., 2.]);
        assert_eq!(lerp.knots.values(), &[0., 1., 2.]);
    }

    #[rstest]
    #[case(-1., -3.)]
    #[case(-0.5, -1.5)]
    #[case(0., 0.)]
    #[case(0.5, 1.5)]
    #[case(1., 3.)]
    #[case(1.5, 2.5)]
    #[case(2., 2.)]
    #[case(2.5, 1.5)]
    #[case(3., 1.)]
    fn test_lerp1d_interp(#[case] input: f64, #[case] expected: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let res = lerp.interp(&input);

        assert_abs_diff_eq!(res, expected, epsilon = eps);
    }

    #[rstest]
    #[case(-1., 3.)]
    #[case(-0.5, 3.)]
    #[case(0., 3.)]
    #[case(0.5, 3.)]
    #[case(1., -1.)]
    #[case(1.5, -1.)]
    #[case(2., -1.)]
    #[case(2.5, -1.)]
    #[case(3., -1.)]
    fn test_lerp1d_der1(#[case] input: f64, #[case] expected: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let res = lerp.der1(&input);

        assert_abs_diff_eq!(res, expected, epsilon = eps);
    }

    #[rstest]
    #[case(-1.)]
    #[case(-0.5)]
    #[case(0.)]
    #[case(0.5)]
    #[case(1.)]
    #[case(1.5)]
    #[case(2.)]
    #[case(2.5)]
    #[case(3.)]
    fn test_lerp1d_der01(#[case] input: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let (der0, der1) = lerp.der01(&input);

        assert_abs_diff_eq!(der0, lerp.eval(&input), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&input), epsilon = eps);
    }

    #[rstest]
    #[case(-1.)]
    #[case(-0.5)]
    #[case(0.)]
    #[case(0.5)]
    #[case(1.)]
    #[case(1.5)]
    #[case(2.)]
    #[case(2.5)]
    #[case(3.)]
    fn test_lerp1d_der2(#[case] input: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let res = lerp.der2(&input);

        assert_abs_diff_eq!(res, 0., epsilon = eps);
    }

    #[rstest]
    #[case(-1.)]
    #[case(-0.5)]
    #[case(0.)]
    #[case(0.5)]
    #[case(1.)]
    #[case(1.5)]
    #[case(2.)]
    #[case(2.5)]
    #[case(3.)]
    fn test_lerp_der012(#[case] input: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let (der0, der1, der2) = lerp.der012(&input);

        assert_abs_diff_eq!(der0, lerp.eval(&input), epsilon = eps);
        assert_abs_diff_eq!(der1, lerp.der1(&input), epsilon = eps);
        assert_abs_diff_eq!(der2, lerp.der2(&input), epsilon = eps);
    }

    #[rstest]
    #[case(-2., -1., -4.5)]
    #[case(-2., 0., -6.)]
    #[case(0., 0.5, 0.375)]
    #[case(-2., 0.5, -5.625)]
    #[case(0., 1., 1.5)]
    #[case(1.5, 2., 1.125)]
    #[case(2., 3., 1.5)]
    #[case(3., 4., 0.5)]
    #[case(1.5, 3.0, 2.625)]
    fn test_integrate(#[case] from: f64, #[case] to: f64, #[case] expected: f64) {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);
        let eps = 1e-15;

        let from_to = lerp.integrate(&from, &to);
        let to_from = lerp.integrate(&to, &from);
        let from_from = lerp.integrate(&from, &from);
        let to_to = lerp.integrate(&to, &to);

        assert_abs_diff_eq!(from_to, expected, epsilon = eps);
        assert_abs_diff_eq!(to_from, -expected, epsilon = eps);
        assert_abs_diff_eq!(from_from, 0., epsilon = eps);
        assert_abs_diff_eq!(to_to, 0., epsilon = eps);
    }

    #[test]
    fn test_lerp1d_destruct() {
        let knots = Series::new(vec![0., 1., 2.], vec![0., 3., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let lerp = Lerp1d::new(knots);

        let (builder, knots) = lerp.destruct();

        assert_eq!(builder, Lerp1dBuilder);
        assert_eq!(knots.grids(), &[0., 1., 2.]);
        assert_eq!(knots.values(), &[0., 3., 2.]);
    }

    #[test]
    fn test_lerp1d_build() {
        let grids: Vec<f64> = vec![0., 1., 2.];
        let values: Vec<f64> = vec![0., 3., 2.];
        let knots = Series::new(grids.clone(), values.clone()).unwrap();

        let lerp = Lerp1dBuilder.build(knots).unwrap();

        assert_eq!(lerp.knots.grids(), grids.as_slice());
        assert_eq!(lerp.knots.values(), values.as_slice());
    }
}
