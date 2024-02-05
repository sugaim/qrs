use std::ops::{Div, Sub};

use num::{One, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    func1d::{Func1dDer1, Func1dDer2},
    num::{RelPos, Vector},
};

use super::{DestructibleInterp1d, Interp1d, Interp1dBuilder, _knots::Knots};

#[derive(Clone, Debug, PartialEq)]
pub struct Lerp1d<G, V> {
    knots: Knots<G, V>,
}

impl<G: PartialOrd, V> Lerp1d<G, V> {
    pub fn new(gs: Vec<G>, vs: Vec<V>) -> Result<Self, anyhow::Error> {
        Knots::new(gs, vs).map(|knots| Self { knots })
    }
}

impl<G: RelPos, V: Vector<G::Output>> Interp1d for Lerp1d<G, V> {
    type Grid = G;
    type Value = V;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Lerp1dBuilder;

impl Default for Lerp1dBuilder {
    fn default() -> Self {
        Self
    }
}

impl<G: RelPos, V: Vector<G::Output>> Interp1dBuilder<G, V> for Lerp1dBuilder {
    type Output = Lerp1d<G, V>;
    type Error = anyhow::Error;

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

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

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
}
