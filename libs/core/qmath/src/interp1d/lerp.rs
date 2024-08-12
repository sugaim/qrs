use std::ops::{Div, Sub};

use anyhow::{anyhow, Context};
use num::One;
use qcollections::{
    flat_map::FlatMap,
    size_ensured::{RequireMinSize, SizeEnsured},
};

use crate::num::{DerX1d, DerXX1d, RelPos, Vector};

use super::{Interp1d, Interp1dBuilder, RebuildableInterp1d};

// -----------------------------------------------------------------------------
// Lerp1d
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(
    deserialize = "X: PartialOrd + serde::Deserialize<'de>, V: serde::Deserialize<'de>"
))]
pub struct Lerp1d<X, V> {
    data: SizeEnsured<FlatMap<X, V>, 2>,
}

impl<X, V> Lerp1d<X, V> {
    #[inline]
    pub fn new(data: SizeEnsured<FlatMap<X, V>, 2>) -> Self {
        Lerp1d { data }
    }
}

impl<X, V> Interp1d for Lerp1d<X, V>
where
    X: RelPos,
    V: Vector<X::Output>,
{
    type X = X;
    type Value = V;

    #[inline]
    fn interpolatee(&self) -> &FlatMap<Self::X, Self::Value> {
        &self.data
    }

    fn interp(&self, x: &X) -> anyhow::Result<Self::Value> {
        let index = self.data.interval_index(x);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;
        let (xl, yl) = self.data.at(index).unwrap();
        let (xr, yr) = self.data.at(index + 1).unwrap();

        let wr = x.relpos_between(xl, xr).unwrap();
        let wl = <X::Output as One>::one() - &wr;
        Ok(yl.clone() * &wl + yr.clone() * &wr)
    }
}

impl<X, V> DerX1d<X> for Lerp1d<X, V>
where
    X: Clone + RelPos + Sub,
    V: Vector<<X as RelPos>::Output> + Div<<X as Sub>::Output>,
{
    type DerX = <V as Div<<X as Sub>::Output>>::Output;

    fn der_x(&self, x: &X) -> anyhow::Result<Self::DerX> {
        let index = self.data.interval_index(x);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;
        let (xl, yl) = self.data.at(index).unwrap();
        let (xr, yr) = self.data.at(index + 1).unwrap();

        let dx = xr.clone() - xl.clone();

        Ok((yr.clone() - yl) / dx)
    }

    fn der_0_x(&self, arg: &X) -> anyhow::Result<(Self::Output, Self::DerX)> {
        let index = self.data.interval_index(arg);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;
        let (xl, yl) = self.data.at(index).unwrap();
        let (xr, yr) = self.data.at(index + 1).unwrap();

        let dx = xr.clone() - xl.clone();

        let wr = arg.relpos_between(xl, xr).unwrap();
        let wl = <<X as RelPos>::Output as One>::one() - &wr;

        let value = yl.clone() * &wl + yr.clone() * &wr;
        let der_x = (yr.clone() - yl) / dx;

        Ok((value, der_x))
    }
}

impl<X, V> DerXX1d<X> for Lerp1d<X, V>
where
    X: Clone + RelPos + Sub,
    V: Vector<<X as RelPos>::Output> + Div<<X as Sub>::Output>,
    <V as Div<<X as Sub>::Output>>::Output: Div<<X as Sub>::Output>,
{
    type DerXX = <<V as Div<<X as Sub>::Output>>::Output as Div<<X as Sub>::Output>>::Output;

    #[inline]
    fn der_xx(&self, x: &X) -> anyhow::Result<Self::DerXX> {
        let index = self.data.interval_index(x);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;

        let (xl, _) = self.data.at(index).unwrap();
        let (xr, _) = self.data.at(index + 1).unwrap();

        Ok(V::zero() / (xr.clone() - xl.clone()) / (xr.clone() - xl.clone()))
    }

    fn der_0_x_xx(&self, arg: &X) -> anyhow::Result<(Self::Output, Self::DerX, Self::DerXX)> {
        let index = self.data.interval_index(arg);
        let index = index.ok_or_else(|| anyhow!("Given argument maybe uncomparable."))?;
        let (xl, yl) = self.data.at(index).unwrap();
        let (xr, yr) = self.data.at(index + 1).unwrap();

        let wr = arg.relpos_between(xl, xr).unwrap();
        let wl = <<X as RelPos>::Output as One>::one() - &wr;

        let value = yl.clone() * &wl + yr.clone() * &wr;
        let der_x = (yr.clone() - yl) / (xr.clone() - xl.clone());
        let der_xx = V::zero() / (xr.clone() - xl.clone()) / (xr.clone() - xl.clone());

        Ok((value, der_x, der_xx))
    }
}

impl<X, V> RebuildableInterp1d for Lerp1d<X, V>
where
    X: RelPos,
    V: Vector<X::Output>,
{
    type Builder = Lerp1dBuilder;

    fn destruct(self) -> (Self::Builder, FlatMap<Self::X, Self::Value>) {
        (Lerp1dBuilder, self.data.into_inner())
    }
}

// -----------------------------------------------------------------------------
// Lerp1dBuilder
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lerp1dBuilder;

impl<X, V> Interp1dBuilder<X, V> for Lerp1dBuilder
where
    X: RelPos,
    V: Vector<X::Output>,
{
    type Output = Lerp1d<X, V>;

    #[inline]
    fn build(self, data: FlatMap<X, V>) -> anyhow::Result<Self::Output> {
        let data = data.require_min_size().context("Building lerp")?;
        Ok(Lerp1d::new(data))
    }
}
