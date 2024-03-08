use std::ops::{Div, Mul, Sub};

use anyhow::ensure;
use derivative::Derivative;
use itertools::{izip, Itertools};
use qrs_collections::{LazyTypedVecBuffer, MinSized, RequireMinSize, Series};

use crate::func1d::{FiniteDiffMethod, Func1dDer1, Func1dDer2};
use crate::interp1d::{DestructibleInterp1d, Interp1d, Interp1dBuilder};
use crate::num::{RelPos, Scalar, Vector};

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
#[derive(Debug, Derivative)]
#[derivative(PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, schemars::JsonSchema),
    serde(bound(
        serialize = "G: serde::Serialize + PartialOrd, V: serde::Serialize, S: serde::Serialize",
    ))
)]
pub struct CHermite1d<G, V, S> {
    knots: MinSized<Series<G, V>, 2>,

    #[derivative(PartialEq = "ignore")]
    #[cfg_attr(feature = "serde", serde(skip))]
    coeffs: Vec<CubicCoeff<V>>,

    #[derivative(PartialEq = "ignore")]
    #[cfg_attr(feature = "serde", serde(skip))]
    slope_buf: LazyTypedVecBuffer,
    scheme: S,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<'de, G, V, S> serde::Deserialize<'de> for CHermite1d<G, V, S>
where
    G: Clone + PartialOrd + Sub + RelPos + serde::Deserialize<'de>,
    V: Vector<<G as RelPos>::Output> + serde::Deserialize<'de>,
    S: CHermiteScheme<G, V> + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<CHermite1d<G, V, S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Data<G: PartialOrd, V, S> {
            knots: MinSized<Series<G, V>, 2>,
            scheme: S,
        }

        let Data { knots, scheme } = Data::deserialize(deserializer)?;
        CHermite1d::new(knots, scheme).map_err(serde::de::Error::custom)
    }
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
        knots: MinSized<Series<G, V>, 2>,
        scheme: S,
        mut coeffs: Vec<CubicCoeff<V>>,
        buf: LazyTypedVecBuffer,
    ) -> Result<Self, anyhow::Error> {
        let mut slopes = buf.into_empty_vec();
        scheme.calc_slope(&mut slopes, &knots)?;
        ensure!(
            slopes.len() == knots.len(),
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
            let ord2 = (vr.clone() - vl) * &f2s(3.) + dyl.clone() * &f2s(-2.) - &dyr;
            let ord3 = (vl.clone() - vr) * &f2s(2.) + &dyl + &dyr;
            coeffs.push(CubicCoeff { ord1, ord2, ord3 });
        }
        Ok(Self {
            knots,
            coeffs,
            slope_buf: LazyTypedVecBuffer::reuse(slopes),
            scheme,
        })
    }

    #[inline]
    pub fn new(knots: MinSized<Series<G, V>, 2>, scheme: S) -> Result<Self, anyhow::Error> {
        Self::_new(knots, scheme, Default::default(), Default::default())
    }
}

//
// methods
//
impl<G, V, S> CHermite1d<G, V, S> {
    #[inline]
    pub fn knots(&self) -> (&[G], &[V]) {
        (self.knots.grids(), self.knots.values())
    }
}

impl<G, V, S> Interp1d for CHermite1d<G, V, S>
where
    G: RelPos,
    V: Vector<G::Output>,
{
    type Grid = G;
    type Value = V;

    fn interp(&self, x: &Self::Grid) -> Self::Value {
        let idx = self.knots.interval_index_of(x).unwrap();
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
        let idx = self.knots.interval_index_of(x).unwrap();
        let CubicCoeff { ord1, ord2, ord3 } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        ((ord3.clone() * &wx(1.5) + ord2) * &wx(2.) + ord1) / dx()
    }

    fn der01(&self, x: &G) -> (Self::Output, Self::Der1) {
        let idx = self.knots.interval_index_of(x).unwrap();
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
        let idx = self.knots.interval_index_of(x).unwrap();
        let CubicCoeff { ord2, ord3, .. } = &self.coeffs[idx];
        let (gl, gr) = (&self.knots.grids()[idx], &self.knots.grids()[idx + 1]);

        let w = x.relpos_between(gl, gr);
        let dx = || gr.clone() - gl.clone();
        let wx = |v: f64| w.clone() * <<G as RelPos>::Output as Scalar>::nearest_value_of(v);
        let f2s = |v: f64| <<G as RelPos>::Output as Scalar>::nearest_value_of(v);

        (ord3.clone() * &wx(3.) + ord2) * &f2s(2.) / dx() / dx()
    }

    fn der012(&self, x: &G) -> (Self::Output, Self::Der1, Self::Der2) {
        let idx = self.knots.interval_index_of(x).unwrap();
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
    V: 'static + Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    type Builer = CHermite1dBuilder<S>;

    #[inline]
    fn destruct(self) -> (Self::Builer, Series<G, V>) {
        let builder = CHermite1dBuilder {
            scheme: self.scheme,
            coeff_buf: self.coeffs.into(),
            slope_buf: self.slope_buf,
        };
        (builder, self.knots.into_inner())
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
    type Slope: 'static + Clone + Mul<G::Output, Output = V>;

    fn calc_slope(
        &self,
        dst: &mut Vec<Self::Slope>,
        knots: &MinSized<Series<G, V>, 2>,
    ) -> Result<(), anyhow::Error>;
}

// -----------------------------------------------------------------------------
// CHermite1dBuilder
//
#[derive(Debug, Derivative)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
#[derivative(PartialEq)]
pub struct CHermite1dBuilder<S> {
    scheme: S,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore")]
    slope_buf: LazyTypedVecBuffer,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore")]
    coeff_buf: LazyTypedVecBuffer,
}

//
// construction
//
impl<S: Clone> Clone for CHermite1dBuilder<S> {
    fn clone(&self) -> Self {
        Self {
            scheme: self.scheme.clone(),
            slope_buf: Default::default(),
            coeff_buf: Default::default(),
        }
    }
}

impl<S> CHermite1dBuilder<S> {
    pub fn new(scheme: S) -> Self {
        Self {
            scheme,
            slope_buf: Default::default(),
            coeff_buf: Default::default(),
        }
    }
}

//
// methods
//
impl<G, V, S> Interp1dBuilder<G, V> for CHermite1dBuilder<S>
where
    G: Clone + PartialOrd + Sub + RelPos,
    V: 'static + Vector<<G as RelPos>::Output>,
    S: CHermiteScheme<G, V>,
{
    type Err = anyhow::Error;
    type Output = CHermite1d<G, V, S>;

    fn build(self, knots: Series<G, V>) -> Result<Self::Output, Self::Err> {
        CHermite1d::_new(
            knots.require_min_size()?,
            self.scheme,
            self.coeff_buf.into_empty_vec(),
            self.slope_buf,
        )
    }
}

// -----------------------------------------------------------------------------
// CatmullRomScheme
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
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
    <V as Div<<G as Sub>::Output>>::Output: 'static + Clone + Mul<<G as Sub>::Output, Output = V>,
{
    type Slope = <V as Div<<G as Sub>::Output>>::Output;

    fn calc_slope(
        &self,
        dst: &mut Vec<Self::Slope>,
        knots: &MinSized<Series<G, V>, 2>,
    ) -> Result<(), anyhow::Error> {
        if dst.capacity() < knots.len() {
            dst.reserve(knots.len() - dst.capacity());
        }
        dst.clear();
        for i in 0..knots.len() {
            let (il, ir) = {
                // unadjusted indices
                let (il, ir) = match self.method {
                    FiniteDiffMethod::Forward => (i, i + 1),
                    FiniteDiffMethod::Backward => (i.max(1) - 1, i),
                    FiniteDiffMethod::Central => (i.max(1) - 1, i + 1),
                };
                (il.clamp(0, knots.len() - 2), ir.clamp(1, knots.len() - 1))
            };
            let (gl, vl) = knots.get(il).unwrap();
            let (gr, vr) = knots.get(ir).unwrap();
            dst.push((vr.clone() - vl) / (gr.clone() - gl.clone()));
        }
        Ok(())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, vec};

    use rstest::rstest;
    use serde::Deserialize;
    use serde_json::from_reader;

    use super::*;

    mockall::mock! {
        Scheme {}

        impl CHermiteScheme<f64, f64> for Scheme {
            type Slope = f64;

            fn calc_slope(
                &self,
                dst: &mut Vec<f64>,
                knots: &MinSized<Series<f64, f64>, 2>,
            ) -> anyhow::Result<()>;
        }
    }

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    // knots which a test case assumes
    #[derive(Deserialize)]
    struct Input {
        xs: Vec<f64>,
        ys: Vec<f64>,
    }

    // expected output data
    #[derive(Deserialize)]
    struct Output {
        #[allow(dead_code)]
        coefficients: Vec<HashMap<String, f64>>,
        evalated: Vec<(f64, f64, f64, f64)>, // x, y, der1, der2
    }

    //
    // common behaviors
    //
    #[test]
    fn test_chermite1d_new() {
        let mut scheme = MockScheme::new();
        scheme.expect_calc_slope().once().returning(|s, _| {
            *s = vec![1., 2., 5.];
            Ok(())
        });
        let knots = Series::new(vec![0., 1., 2.], vec![0., 1., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();

        let mut interp = CHermite1d::new(knots, scheme).unwrap();

        assert_eq!(interp.knots().0, &[0., 1., 2.]);
        assert_eq!(interp.knots().1, &[0., 1., 2.]);
        interp.scheme.checkpoint();
    }

    #[test]
    fn test_chermite1d_builder() {
        let mut scheme = MockScheme::new();
        scheme.expect_calc_slope().once().returning(|s, _| {
            *s = vec![1., 2., 5.];
            Ok(())
        });
        let knots = Series::new(vec![0., 1., 2.], vec![0., 1., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let expected = CHermite1d::new(knots.clone(), scheme).unwrap();
        let mut scheme = MockScheme::new();
        scheme.expect_calc_slope().once().returning(|s, _| {
            *s = vec![1., 2., 5.];
            Ok(())
        });
        let builder = CHermite1dBuilder::new(scheme);

        let mut interp = builder.build(knots.into_inner()).unwrap();

        assert_eq!(interp.knots, expected.knots);
        assert_eq!(interp.coeffs, expected.coeffs);
        interp.scheme.checkpoint();
    }

    #[test]
    fn test_chermite1d_destruct() {
        let mut scheme = MockScheme::new();
        scheme.expect_calc_slope().times(2).returning(|s, _| {
            *s = vec![1., 2., 5.];
            Ok(())
        });
        let knots = Series::new(vec![0., 1., 2.], vec![0., 1., 2.])
            .unwrap()
            .require_min_size()
            .unwrap();
        let interp = CHermite1d::new(knots.clone(), scheme).unwrap();
        let coeffs = interp.coeffs.clone();

        let (builder, knots) = interp.destruct();
        let mut rebuild = builder.build(knots.clone()).unwrap();

        assert_eq!(rebuild.knots.as_ref(), &knots);
        assert_eq!(rebuild.coeffs, coeffs);
        rebuild.scheme.checkpoint();
    }

    //
    // CatmullRom specifics
    //
    #[rstest]
    #[case(Series::new(vec![0., 1., 2.], vec![0., 1., 2.]).unwrap(), FiniteDiffMethod::Forward, vec![1., 1., 1.])]
    #[case(Series::new(vec![0., 1., 2.], vec![0., 1., 2.]).unwrap(), FiniteDiffMethod::Backward, vec![1., 1., 1.])]
    #[case(Series::new(vec![0., 1., 2.], vec![0., 1., 2.]).unwrap(), FiniteDiffMethod::Central, vec![1., 1., 1.])]
    #[case(Series::new(vec![0., 2., 3., 7.], vec![0., 4., 3., 5.]).unwrap(), FiniteDiffMethod::Forward, vec![2., -1., 0.5, 0.5])]
    #[case(Series::new(vec![0., 2., 3., 7.], vec![0., 4., 3., 5.]).unwrap(), FiniteDiffMethod::Backward, vec![2., 2., -1., 0.5])]
    #[case(Series::new(vec![0., 2., 3., 7.], vec![0., 4., 3., 5.]).unwrap(), FiniteDiffMethod::Central, vec![2., 1., 0.2, 0.5])]
    fn test_cr_scheme(
        #[case] knots: Series<f64, f64>,
        #[case] scheme: FiniteDiffMethod,
        #[case] expected: Vec<f64>,
    ) {
        let scheme = CatmullRomScheme::new(scheme);
        let knots = knots.require_min_size().unwrap();
        let mut slopes = Vec::new();

        scheme.calc_slope(&mut slopes, &knots).unwrap();

        assert_eq!(slopes, expected);
    }

    #[rstest]
    #[case(FiniteDiffMethod::Forward)]
    #[case(FiniteDiffMethod::Backward)]
    #[case(FiniteDiffMethod::Central)]
    fn test_cr_spline(#[case] scheme: FiniteDiffMethod) {
        let sch = CatmullRomScheme::new(scheme);
        let name = match scheme {
            FiniteDiffMethod::Forward => "fwd",
            FiniteDiffMethod::Backward => "bwd",
            FiniteDiffMethod::Central => "cen",
        };
        let mut test_data_dir = crate_root();
        test_data_dir.push("testdata/interp1d");
        let mut inpath = test_data_dir.clone();
        inpath.push(format!("chermite.CatmullRom.{}.in.json", name));
        let mut outpath = test_data_dir.clone();
        outpath.push(format!("chermite.CatmullRom.{}.out.json", name));
        let mut serialized = test_data_dir.clone();
        serialized.push(format!("chermite.CatmullRom.{}.serialized.json", name));

        let input: Input = from_reader(std::fs::File::open(inpath).unwrap()).unwrap();
        let knots = Series::new(input.xs, input.ys)
            .unwrap()
            .require_min_size()
            .unwrap();
        let expected: Output = from_reader(std::fs::File::open(outpath).unwrap()).unwrap();

        let mut slopes = Vec::new();
        sch.calc_slope(&mut slopes, &knots).unwrap();
        let interp = CHermite1d::new(knots, sch).unwrap();

        for (x, y, der1, der2) in expected.evalated {
            let tested = interp.interp(&x);
            assert!(
                (tested - y).abs() < 1e-10,
                "{name}:\n\t    x = {x}\n\ty.exp = {y}\n\ty.tst = {tested}"
            );
            let tested = interp.der1(&x);
            assert!(
                (tested - der1).abs() < 1e-10,
                "{name}:\n\t    x = {x}\n\tder1.exp = {der1}\n\tder1.tst = {tested}"
            );
            let tested = interp.der2(&x);
            assert!(
                (tested - der2).abs() < 1e-10,
                "{name}:\n\t    x = {x}\n\tder2.exp = {der2}\n\tder2.tst = {tested}"
            );
            let (tested, tested_der1) = interp.der01(&x);
            assert!(
                (tested - y).abs() < 1e-10,
                "{name}/der01:\n\t    x = {x}\n\tder01.exp = {y}\n\tder01.tst = {y}"
            );
            assert!(
                (der1 - tested_der1).abs() < 1e-10,
                "{name}/der01:\n\t    x = {x}\n\tder01.exp = {der1}\n\tder01.tst = {der1}"
            );
            let (tested, tested_der1, tested_der2) = interp.der012(&x);
            assert!(
                (tested - y).abs() < 1e-10,
                "{name}/der012:\n\t    x = {x}\n\tder012.exp = {y}\n\tder012.tst = {y}"
            );
            assert!(
                (der1 - tested_der1).abs() < 1e-10,
                "{name}/der012:\n\t    x = {x}\n\tder012.exp = {der1}\n\tder012.tst = {der1}"
            );
            assert!(
                (der2 - tested_der2).abs() < 1e-10,
                "{name}/der012:\n\t    x = {x}\n\tder012.exp = {der2}\n\tder012.tst = {der2}"
            );
        }
    }
}
