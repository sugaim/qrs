use std::ops::{Div, Sub};

use anyhow::{anyhow, Context};
use num::Zero;
use qcollections::{
    flat_map::FlatMap,
    size_ensured::{RequireMinSize, SizeEnsured, SizedContainer},
};

use crate::num::{DerX1d, DerXX1d, RelPos, Vector};

use super::{Interp1d, Interp1dBuilder, RebuildableInterp1d};

// -----------------------------------------------------------------------------
// Pwconst1d
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(
    deserialize = "X: PartialOrd + serde::Deserialize<'de>, V: serde::Deserialize<'de>"
))]
pub struct Pwconst1d<X, V> {
    data: SizeEnsured<FlatMap<X, V>, 2>,
}

impl<X, V> Pwconst1d<X, V> {
    #[inline]
    pub fn new(data: SizeEnsured<FlatMap<X, V>, 2>) -> Self {
        Pwconst1d { data }
    }
}

impl<X, V> Interp1d for Pwconst1d<X, V>
where
    X: PartialOrd,
    V: Clone,
{
    type X = X;
    type Value = V;

    #[inline]
    fn interpolatee(&self) -> &FlatMap<Self::X, Self::Value> {
        &self.data
    }

    #[inline]
    fn interp(&self, x: &X) -> anyhow::Result<Self::Value> {
        let (xlast, ylast) = self.data.inner().iter().last().unwrap();
        if xlast <= x {
            return Ok(ylast.clone());
        }
        let index = self.data.interval_index(x);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;
        Ok(self.data.at(index).unwrap().1.clone())
    }
}

impl<X, V> DerX1d<X> for Pwconst1d<X, V>
where
    X: PartialOrd + Sub + Clone,
    X::Output: Zero + Clone,
    V: Clone + Zero + Div<<X as Sub>::Output>,
{
    type DerX = <V as Div<<X as Sub>::Output>>::Output;

    #[inline]
    fn der_x(&self, _: &X) -> anyhow::Result<Self::DerX> {
        let xfirst = self.data.inner().iter().next().unwrap().0;
        let xlast = self.data.inner().iter().last().unwrap().0;
        let dx = xlast.clone() - xfirst.clone();
        assert!(!dx.is_zero());
        Ok(V::zero() / dx)
    }
}

impl<X, V> DerXX1d<X> for Pwconst1d<X, V>
where
    X: PartialOrd + Sub + Clone,
    X::Output: Zero + Clone,
    V: Clone + Zero + Div<<X as Sub>::Output>,
    <V as Div<<X as Sub>::Output>>::Output: Div<<X as Sub>::Output>,
{
    type DerXX = <<V as Div<<X as Sub>::Output>>::Output as Div<<X as Sub>::Output>>::Output;

    #[inline]
    fn der_xx(&self, _: &X) -> Result<Self::DerXX, Self::Error> {
        let xfirst = self.data.inner().iter().next().unwrap().0;
        let xlast = self.data.inner().iter().last().unwrap().0;
        let dx = xlast.clone() - xfirst.clone();
        assert!(!dx.is_zero());
        Ok(V::zero() / dx.clone() / dx)
    }
}

impl<X, V> RebuildableInterp1d for Pwconst1d<X, V>
where
    X: RelPos,
    V: Vector<X::Output>,
{
    type Builder = Pwconst1dBuilder;

    #[inline]
    fn destruct(self) -> (Self::Builder, FlatMap<Self::X, Self::Value>) {
        (Pwconst1dBuilder, self.data.into_inner())
    }
}

// -----------------------------------------------------------------------------
// Pwconst1dBuilder
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pwconst1dBuilder;

impl<X, V> Interp1dBuilder<X, V> for Pwconst1dBuilder
where
    X: RelPos,
    V: Vector<X::Output>,
{
    type Output = Pwconst1d<X, V>;

    #[inline]
    fn build(self, data: FlatMap<X, V>) -> anyhow::Result<Self::Output> {
        let data = data.require_min_size().context("Building pwconst interp")?;
        Ok(Pwconst1d::new(data))
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, vec};

    use crate::num::Func1d;

    use super::*;

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn test_pwconst1d_eval() {
        let indata = crate_root().join("testdata/interp1d/in/pwconst1d.json");
        let indata: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(indata).unwrap()).unwrap();
        let indata = indata.get("interp").unwrap().clone();
        let tested: Pwconst1d<f64, f64> = serde_json::from_value(indata).unwrap();

        let expected = crate_root().join("testdata/interp1d/out/pwconst1d.csv");
        let expected = std::fs::read_to_string(expected).unwrap();

        for line in expected.split('\n').skip(1) {
            let vals = line.split(',').collect::<Vec<_>>();
            let x: f64 = vals[0].parse().unwrap();
            let y: f64 = vals[1].parse().unwrap();
            let dy: f64 = vals[2].parse().unwrap();
            let d2y: f64 = vals[3].parse().unwrap();

            let y_ = tested.interp(&x).unwrap();
            approx::assert_abs_diff_eq!(y, y_, epsilon = 1e-6);

            let y_ = tested.eval(&x).unwrap();
            approx::assert_abs_diff_eq!(y, y_, epsilon = 1e-6);

            let dy_ = tested.der_x(&x).unwrap();
            approx::assert_abs_diff_eq!(dy, dy_, epsilon = 1e-6);

            let d2y_ = tested.der_xx(&x).unwrap();
            approx::assert_abs_diff_eq!(d2y, d2y_, epsilon = 1e-6);

            let (y_, dy_) = tested.der_0_x(&x).unwrap();
            approx::assert_abs_diff_eq!(y, y_, epsilon = 1e-6);
            approx::assert_abs_diff_eq!(dy, dy_, epsilon = 1e-6);

            let (y_, dy_, d2y_) = tested.der_0_x_xx(&x).unwrap();

            approx::assert_abs_diff_eq!(y, y_, epsilon = 1e-6);
            approx::assert_abs_diff_eq!(dy, dy_, epsilon = 1e-6);
            approx::assert_abs_diff_eq!(d2y, d2y_, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_bulder() {
        let xs = vec![0.0, 1.0, 2.0];
        let ys = vec![0.0, 1.0, 2.0];
        let data = FlatMap::with_data(xs, ys).unwrap();

        let interp = Pwconst1dBuilder.build(data.clone()).unwrap();

        assert_eq!(interp.interpolatee(), &data);
    }

    #[test]
    fn test_builder_err() {
        let xs = vec![1.0];
        let ys = vec![1.0];
        let data = FlatMap::with_data(xs, ys).unwrap();

        let res = Pwconst1dBuilder.build(data);

        assert!(res.is_err());
    }

    #[test]
    fn test_destruct() {
        let xs = vec![0.0, 1.0, 2.0];
        let ys = vec![0.0, 1.0, 2.0];
        let data = FlatMap::with_data(xs, ys).unwrap();

        let interp = Pwconst1dBuilder.build(data.clone()).unwrap();
        let (builder, data) = interp.destruct();

        assert_eq!(builder, Pwconst1dBuilder);
        assert_eq!(data, data);
    }
}
