// -----------------------------------------------------------------------------
// CubicCoeff
//

use std::ops::{Div, Mul, Sub};

use anyhow::ensure;
use derivative::Derivative;
use itertools::{izip, Itertools};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    collection::VecBuffer,
    func1d::{FiniteDiffMethod, Func1dDer1, Func1dDer2},
    num::{RelPos, Scalar, Vector},
};

use super::{DestructibleInterp1d, Interp1d, Interp1dBuilder, _knots::Knots};

// -----------------------------------------------------------------------------
// CubicCoeff
//

/// Coefficients for a cubic polynomial.
#[derive(Debug, Clone, PartialEq)]
struct CubicCoeff<V> {
    pub ord1: V,
    pub ord2: V,
    pub ord3: V,
}

// -----------------------------------------------------------------------------
// CHermite1d
//
#[derive(Debug, Derivative, Serialize, Deserialize, JsonSchema)]
#[derivative(PartialEq)]
pub struct CHermite1d<G, V, S> {
    #[serde(bound(
        serialize = "G: PartialOrd + Serialize, V: Serialize",
        deserialize = "G: PartialOrd + Deserialize<'de>, V: Deserialize<'de>"
    ))]
    knots: Knots<G, V>,
    #[serde(skip)]
    coeffs: Vec<CubicCoeff<V>>,
    #[serde(skip)]
    #[derivative(PartialEq = "ignore")]
    slope_buf: VecBuffer,
    scheme: S,
}

//
// construction
//
impl<G, V, S> Clone for CHermite1d<G, V, S>
where
    G: Clone,
    V: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            knots: self.knots.clone(),
            coeffs: self.coeffs.clone(),
            slope_buf: Default::default(),
            scheme: self.scheme.clone(),
        }
    }
}

impl<G, V, S> CHermite1d<G, V, S>
where
    G: Clone + PartialOrd + Sub + RelPos,
    V: Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    /// Implementation of the constructor.
    ///
    /// To reuse allocated memories, this method takes a buffer for the slopes
    /// and a vector for the coefficients.
    fn _new(
        gs: Vec<G>,
        vs: Vec<V>,
        scheme: S,
        mut coeffs: Vec<CubicCoeff<V>>,
        buf: VecBuffer,
    ) -> Result<Self, anyhow::Error> {
        let knots = Knots::new(gs, vs)?;
        let mut slopes = buf.into_vec();
        scheme.calc_slope(&mut slopes, &knots.grids(), &knots.values())?;
        ensure!(
            slopes.len() == knots.grids().len(),
            "The number of slopes must be the same as the number of knots."
        );
        let f2s = |v: f64| <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        let gs = knots.grids().iter().tuple_windows();
        let vs = knots.values().iter().tuple_windows();
        let ss = slopes.iter().tuple_windows();
        coeffs.clear();
        for ((gl, gr), (vl, vr), (sl, sr)) in izip!(gs, vs, ss) {
            let (dyl, dyr) = (
                sl.clone() * (gr.clone() - gl.clone()),
                sr.clone() * (gr.clone() - gl.clone()),
            );
            let ord1 = dyl.clone();
            let ord2 = (vr.clone() - &vl) * &f2s(3.) + dyl.clone() * &f2s(-2.) - &dyr;
            let ord3 = (vl.clone() - &vr) * &f2s(2.) + &dyl + &dyr;
            coeffs.push(CubicCoeff { ord1, ord2, ord3 });
        }
        Ok(Self {
            knots,
            coeffs,
            slope_buf: VecBuffer::reuse(slopes),
            scheme,
        })
    }

    #[inline]
    pub fn new(gs: Vec<G>, vs: Vec<V>, scheme: S) -> Result<Self, anyhow::Error> {
        Self::_new(gs, vs, scheme, Default::default(), Default::default())
    }
}

//
// methods
//
impl<G, V, S> Interp1d for CHermite1d<G, V, S>
where
    G: RelPos,
    V: Vector<G::Output>,
{
    type Grid = G;
    type Value = V;

    #[inline]
    fn knots(&self) -> (&[Self::Grid], &[Self::Value]) {
        (&self.knots.grids(), &self.knots.values())
    }

    fn interp(&self, x: &Self::Grid) -> Self::Value {
        let idx = self.knots.interval_index_of(x);
        let ord0 = &self.knots.values()[idx];
        let CubicCoeff { ord1, ord2, ord3 } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);

        ((ord3.clone() * &w + ord2) * &w + ord1) * &w + ord0
    }
}

impl<G, V, S> Func1dDer1<G> for CHermite1d<G, V, S>
where
    G: Clone + RelPos + Sub,
    V: Vector<<G as RelPos>::Output> + Div<<G as Sub>::Output>,
{
    type Der1 = <V as Div<<G as Sub>::Output>>::Output;

    fn der1(&self, x: &G) -> Self::Der1 {
        let idx = self.knots.interval_index_of(x);
        let CubicCoeff { ord1, ord2, ord3 } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        ((ord3.clone() * &wx(1.5) + ord2) * &wx(2.) + ord1) / dx()
    }

    fn der01(&self, x: &G) -> (Self::Output, Self::Der1) {
        let idx = self.knots.interval_index_of(x);
        let ord0 = &self.knots.values()[idx];
        let CubicCoeff { ord1, ord2, ord3 } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        (
            ((ord3.clone() * &w + ord2) * &w + ord1) * &w + ord0,
            ((ord3.clone() * &wx(1.5) + ord2) * &wx(2.) + ord1) / dx(),
        )
    }
}

impl<G, V, S> Func1dDer2<G> for CHermite1d<G, V, S>
where
    G: Clone + RelPos + Sub,
    V: Vector<<G as RelPos>::Output> + Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Div<<G as Sub>::Output>,
{
    type Der2 = <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output;

    fn der2(&self, x: &G) -> Self::Der2 {
        let idx = self.knots.interval_index_of(x);
        let CubicCoeff { ord2, ord3, .. } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);
        let f2s = |v: f64| <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        (ord3.clone() * &wx(3.) + ord2) * &f2s(2.) / dx() / dx()
    }

    fn der012(&self, x: &G) -> (Self::Output, Self::Der1, Self::Der2) {
        let idx = self.knots.interval_index_of(x);
        let ord0 = &self.knots.values()[idx];
        let CubicCoeff { ord1, ord2, ord3 } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);
        let f2s = |v: f64| <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        (
            ((ord3.clone() * &w + ord2) * &w + ord1) * &w + ord0,
            ((ord3.clone() * &wx(1.5) + ord2) * &wx(2.) + ord1) / dx(),
            (ord3.clone() * &wx(3.) + ord2) * &f2s(2.) / dx() / dx(),
        )
    }
}

impl<G, V, S> DestructibleInterp1d for CHermite1d<G, V, S>
where
    G: Clone + PartialOrd + Sub + RelPos,
    V: Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    type Builer = BufferReusedCHermite1dBuilder<V, S>;

    fn destruct(self) -> (Self::Builer, Vec<Self::Grid>, Vec<Self::Value>) {
        let (gs, vs) = self.knots.destruct();
        let builder = BufferReusedCHermite1dBuilder {
            scheme: self.scheme,
            coeffs: self.coeffs,
            slope_buf: self.slope_buf,
        };
        (builder, gs, vs)
    }
}

// -----------------------------------------------------------------------------
// CHermiteScheme
//

/// Characterization of cubic hermite spline.
///
/// Cubic hermite spline is characterized by the slopes at the knots.
/// This trait behaves as a interpolation scheme for cubic hermite spline by
/// providing the way to calculate the slopes at the knots.
pub trait CHermiteScheme<G: Sub, V> {
    type Slope: Clone + Mul<G::Output, Output = V>;

    fn calc_slope(
        &self,
        dst: &mut Vec<Self::Slope>,
        grids: &[G],
        values: &[V],
    ) -> Result<(), anyhow::Error>;
}

// -----------------------------------------------------------------------------
// CHermite1dBuilder
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CHermite1dBuilder<S> {
    scheme: S,
}

//
// construction
//
impl<S> CHermite1dBuilder<S> {
    pub fn new(scheme: S) -> Self {
        Self { scheme }
    }
}

//
// methods
//
impl<G, V, S> Interp1dBuilder<G, V> for CHermite1dBuilder<S>
where
    G: Clone + PartialOrd + Sub + RelPos,
    V: Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    type Err = anyhow::Error;
    type Output = CHermite1d<G, V, S>;

    fn build(self, grids: Vec<G>, values: Vec<V>) -> Result<Self::Output, Self::Err> {
        CHermite1d::new(grids, values, self.scheme)
    }
}

// -----------------------------------------------------------------------------
// BufferReusedCHermite1dBuilder
//
#[derive(Debug, Derivative, Serialize, Deserialize, JsonSchema)]
#[derivative(PartialEq)]
pub struct BufferReusedCHermite1dBuilder<V, S> {
    scheme: S,
    #[serde(skip)]
    coeffs: Vec<CubicCoeff<V>>,
    #[serde(skip)]
    #[derivative(PartialEq = "ignore")]
    slope_buf: VecBuffer,
}

//
// construction
//
impl<V, S> Clone for BufferReusedCHermite1dBuilder<V, S>
where
    V: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            scheme: self.scheme.clone(),
            coeffs: self.coeffs.clone(),
            slope_buf: Default::default(),
        }
    }
}

//
// methods
//
impl<V, S> BufferReusedCHermite1dBuilder<V, S> {
    #[inline]
    pub fn without_buffer(self) -> CHermite1dBuilder<S> {
        CHermite1dBuilder::new(self.scheme)
    }
}

impl<G, V, S> Interp1dBuilder<G, V> for BufferReusedCHermite1dBuilder<V, S>
where
    G: Clone + PartialOrd + Sub + RelPos,
    V: Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    type Err = anyhow::Error;
    type Output = CHermite1d<G, V, S>;

    fn build(self, grids: Vec<G>, values: Vec<V>) -> Result<Self::Output, Self::Err> {
        CHermite1d::_new(grids, values, self.scheme, self.coeffs, self.slope_buf)
    }
}

// -----------------------------------------------------------------------------
// CatmullRomScheme
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CatmullRomScheme {
    method: FiniteDiffMethod,
}

//
// construction
//
impl CatmullRomScheme {
    pub fn new(method: FiniteDiffMethod) -> Self {
        Self { method }
    }
}

//
// methods
//
impl<G, V> CHermiteScheme<G, V> for CatmullRomScheme
where
    G: Clone + Sub + RelPos,
    V: Vector<<G as RelPos>::Output> + Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Clone + Mul<<G as Sub>::Output, Output = V>,
{
    type Slope = <V as Div<<G as Sub>::Output>>::Output;

    fn calc_slope(
        &self,
        dst: &mut Vec<Self::Slope>,
        grids: &[G],
        values: &[V],
    ) -> Result<(), anyhow::Error> {
        ensure!(
            grids.len() == values.len(),
            "The number of grids and values must be the same."
        );
        ensure!(
            grids.windows(2).all(|w| w[0] < w[1]),
            "The grids must be sorted in ascending order."
        );
        ensure!(2 <= grids.len(), "The number of knots must be at least 2.");
        if dst.capacity() < grids.len() {
            dst.reserve(grids.len() - dst.capacity());
        }
        dst.clear();
        for i in 0..grids.len() {
            let (il, ir) = {
                // unadjusted indices
                let (il, ir) = match self.method {
                    FiniteDiffMethod::Forward => (i, i + 1),
                    FiniteDiffMethod::Backward => (i.max(1) - 1, i),
                    FiniteDiffMethod::Central => (i.max(1) - 1, i + 1),
                };
                (il.clamp(0, grids.len() - 2), ir.clamp(1, grids.len() - 1))
            };
            let (gl, gr) = (&grids[il], &grids[ir]);
            let (vl, vr) = (&values[il], &values[ir]);
            dst.push((vr.clone() - &vl) / (gr.clone() - gl.clone()));
        }
        Ok(())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::*;

    struct AlwaysFailScheme;

    impl CHermiteScheme<f64, f64> for AlwaysFailScheme {
        type Slope = f64;

        fn calc_slope(
            &self,
            _dst: &mut Vec<Self::Slope>,
            grids: &[f64],
            values: &[f64],
        ) -> Result<(), anyhow::Error> {
            Err(anyhow::anyhow!(
                "grids.len={}, values.len={}",
                grids.len(),
                values.len()
            ))
        }
    }

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn test_chermite1d_new() {
        let scheme = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let interp = CHermite1d::new(vec![0., 1., 2.], vec![0., 1., 2.], scheme).unwrap();
        assert_eq!(interp.knots().0, &[0., 1., 2.]);
        assert_eq!(interp.knots().1, &[0., 1., 2.]);

        // errors
        let scheme = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let interp = CHermite1d::new(vec![0., 1.], vec![0., 1., 2.], scheme);
        assert!(interp.is_err());

        let scheme = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let interp = CHermite1d::new(vec![0.], vec![0.], scheme);
        assert!(interp.is_err());

        let scheme = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let interp = CHermite1d::new(Vec::<f64>::new(), Vec::<f64>::new(), scheme);
        assert!(interp.is_err());

        let scheme = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let interp = CHermite1d::new(vec![0., 1., 1.], vec![0., 1., 2.], scheme);
        assert!(interp.is_err());

        let scheme = AlwaysFailScheme;
        let interp = CHermite1d::new(vec![0., 1., 2.], vec![0., 1., 2.], scheme);
        assert!(interp.is_err());
        assert_eq!(
            interp.err().unwrap().to_string(),
            "grids.len=3, values.len=3"
        );
    }

    #[test]
    fn test_cr_scheme() {
        let fwd = CatmullRomScheme::new(FiniteDiffMethod::Forward);
        let bwd = CatmullRomScheme::new(FiniteDiffMethod::Backward);
        let cen = CatmullRomScheme::new(FiniteDiffMethod::Central);
        let mut slopes = Vec::new();

        // case 1
        let grids = vec![0., 1., 2.];
        let values = vec![0., 1., 2.];
        fwd.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![1., 1., 1.]);
        bwd.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![1., 1., 1.]);
        cen.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![1., 1., 1.]);

        // case 2
        let grids = vec![0., 2., 3., 7.];
        let values = vec![0., 4., 3., 5.];
        fwd.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![2., -1., 0.5, 0.5]);
        bwd.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![2., 2., -1., 0.5]);
        cen.calc_slope(&mut slopes, &grids, &values).unwrap();
        assert_eq!(slopes, vec![2., 1., 0.2, 0.5]);

        // errors
        let current_slopes = slopes.clone(); // to check the buffer is not modified when an error occurs

        let grids = vec![0., 1., 2.];
        let values = vec![0., 1., 2., 3.]; // different length
        assert!(fwd.calc_slope(&mut slopes, &grids, &values).is_err());
        assert_eq!(slopes, current_slopes);

        let grids = vec![0.];
        let values = vec![0.]; // too few knots
        assert!(fwd.calc_slope(&mut slopes, &grids, &values).is_err());
        assert_eq!(slopes, current_slopes);

        let grids: Vec<f64> = vec![];
        let values: Vec<f64> = vec![]; // too few knots
        assert!(fwd.calc_slope(&mut slopes, &grids, &values).is_err());
        assert_eq!(slopes, current_slopes);

        let grids = vec![0., 1., 1.]; // not sorted
        let values = vec![0., 1., 2.];
        assert!(fwd.calc_slope(&mut slopes, &grids, &values).is_err());
        assert_eq!(slopes, current_slopes);
    }

    #[derive(Deserialize)]
    struct Input {
        xs: Vec<f64>,
        ys: Vec<f64>,
    }

    #[derive(Deserialize)]
    struct Output {
        #[allow(dead_code)]
        coefficients: Vec<HashMap<String, f64>>,
        evalated: Vec<(f64, f64, f64, f64)>, // x, y, der1, der2
    }

    #[test]
    fn test_cr_spline() {
        let cases = [
            ("fwd", CatmullRomScheme::new(FiniteDiffMethod::Forward)),
            ("bwd", CatmullRomScheme::new(FiniteDiffMethod::Backward)),
            ("cen", CatmullRomScheme::new(FiniteDiffMethod::Central)),
        ];

        let mut test_data_dir = crate_root();
        test_data_dir.push("testdata/interp1d");
        for (name, sch) in cases {
            let mut inpath = test_data_dir.clone();
            inpath.push(format!("chermite.CatmullRom.{}.in.json", name));
            let mut outpath = test_data_dir.clone();
            outpath.push(format!("chermite.CatmullRom.{}.out.json", name));

            let input: Input =
                serde_json::from_reader(std::fs::File::open(inpath).unwrap()).unwrap();
            let expected: Output =
                serde_json::from_reader(std::fs::File::open(outpath).unwrap()).unwrap();

            let mut slopes = Vec::new();
            sch.calc_slope(&mut slopes, &input.xs, &input.ys).unwrap();
            let interp = CHermite1d::new(input.xs, input.ys, sch).unwrap();

            for (x, y, der1, der2) in expected.evalated {
                let tested = interp.interp(&x);
                assert!(
                    (tested - &y).abs() < 1e-10,
                    "{name}:\n\t    x = {x}\n\ty.exp = {y}\n\ty.tst = {tested}"
                );
                let tested = interp.der1(&x);
                assert!(
                    (tested - &der1).abs() < 1e-10,
                    "{name}:\n\t    x = {x}\n\tder1.exp = {der1}\n\tder1.tst = {tested}"
                );
                let tested = interp.der2(&x);
                assert!(
                    (tested - &der2).abs() < 1e-10,
                    "{name}:\n\t    x = {x}\n\tder2.exp = {der2}\n\tder2.tst = {tested}"
                );
                let (y, der1) = interp.der01(&x);
                assert!(
                    (y - &y).abs() < 1e-10,
                    "{name}/der01:\n\t    x = {x}\n\tder01.exp = {y}\n\tder01.tst = {y}"
                );
                assert!(
                    (der1 - &der1).abs() < 1e-10,
                    "{name}/der01:\n\t    x = {x}\n\tder01.exp = {der1}\n\tder01.tst = {der1}"
                );
                let (y, der1, der2) = interp.der012(&x);
                assert!(
                    (y - &y).abs() < 1e-10,
                    "{name}/der012:\n\t    x = {x}\n\tder012.exp = {y}\n\tder012.tst = {y}"
                );
                assert!(
                    (der1 - &der1).abs() < 1e-10,
                    "{name}/der012:\n\t    x = {x}\n\tder012.exp = {der1}\n\tder012.tst = {der1}"
                );
                assert!(
                    (der2 - &der2).abs() < 1e-10,
                    "{name}/der012:\n\t    x = {x}\n\tder012.exp = {der2}\n\tder012.tst = {der2}"
                );
            }
        }
    }
}
