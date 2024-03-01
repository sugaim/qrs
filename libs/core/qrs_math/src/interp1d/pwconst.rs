use std::ops::{Div, Mul, Sub};

#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::func1d::{Func1dDer1, Func1dDer2, Func1dIntegrable, SemiContinuity};
use crate::interp1d::_knots::Knots;
use crate::interp1d::{DestructibleInterp1d, Interp1d, Interp1dBuilder};
use crate::num::{Arithmetic, PartialOrdMinMax, RelPos, Scalar, Vector, Zero};

// -----------------------------------------------------------------------------
// PwConst1d
//

/// 1-dimensional linear interpolation.
///
/// # Example
/// ```
/// use qrs_math::interp1d::Interp1d;
/// use qrs_math::func1d::SemiContinuity;
///
/// let grids = vec![0.0, 1.0, 2.0];
/// let values = vec![0.0, 1.0, 0.0];
/// let cont = SemiContinuity::LeftContinuous;
/// let partition_ratio = 0.5;
/// let interp = qrs_math::interp1d::PwConst1d::new(grids, values, cont, partition_ratio).unwrap();
///
/// assert_eq!(interp.interp(&0.0), 0.0);
/// assert_eq!(interp.interp(&0.5), 0.0);
/// assert_eq!(interp.interp(&0.5001), 1.0);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// assert_eq!(interp.interp(&1.5), 1.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, JsonSchema))]
pub struct PwConst1d<G, V> {
    /// Knots which are interpolated.
    #[cfg_attr(
        feature = "serde",
        serde(bound(serialize = "G: Serialize + PartialOrd, V: Serialize"))
    )]
    knots: Knots<G, V>,
    /// Continuity of the interpolated function.
    continuity: SemiContinuity,
    /// Ratio determining partition point to use the left or right value. (0.0 <= partition <= 1.0)
    partition_ratio: f64,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<'de, G, V> Deserialize<'de> for PwConst1d<G, V>
where
    G: Deserialize<'de> + PartialOrd,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<PwConst1d<G, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data<G, V> {
            #[serde(bound(deserialize = "G: Deserialize<'de> + PartialOrd, V: Deserialize<'de>"))]
            knots: Knots<G, V>,
            continuity: SemiContinuity,
            partition_ratio: f64,
        }
        let data = Data::deserialize(deserializer)?;
        let (gs, vs) = data.knots.destruct();
        Self::new(gs, vs, data.continuity, data.partition_ratio).map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl<G: PartialOrd, V> PwConst1d<G, V> {
    /// Create a new `PwConst1d` interpolation.
    ///
    /// # Errors
    /// - If the length of `gs` is less than 2.
    /// - If the length of `gs` and `vs` are not equal.
    /// - If `gs` is not sorted in ascending order.
    #[inline]
    pub fn new(
        gs: Vec<G>,
        vs: Vec<V>,
        cont: SemiContinuity,
        partition: f64,
    ) -> Result<Self, anyhow::Error> {
        let knots = Knots::new(gs, vs)?;
        if !(0. ..=1.).contains(&partition) {
            return Err(anyhow::anyhow!("partition must be in [0, 1]"));
        }
        Ok(Self {
            knots,
            continuity: cont,
            partition_ratio: partition,
        })
    }
}

//
// methods
//
impl<G, V> PwConst1d<G, V> {
    #[inline]
    pub fn knots(&self) -> (&[G], &[V]) {
        (self.knots.grids(), self.knots.values())
    }
}

impl<G: RelPos, V: Vector<G::Output>> Interp1d for PwConst1d<G, V> {
    type Grid = G;
    type Value = V;

    fn interp(&self, x: &G) -> V {
        let idx = self.knots.interval_index_of(x);
        let (gl, vl) = self.knots.force_get(idx);
        let (gr, vr) = self.knots.force_get(idx + 1);

        let sep = <G::Output as Scalar>::nearest_value_of(self.partition_ratio);

        let w = x.relpos_between(gl, gr);
        if w < sep {
            vl.clone()
        } else if sep < w {
            vr.clone()
        } else {
            match self.continuity {
                SemiContinuity::LeftContinuous => vl.clone(),
                SemiContinuity::RightContinuous => vr.clone(),
            }
        }
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>> Func1dDer1<G> for PwConst1d<G, V>
where
    G: Clone + Sub<G>,
    V: Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Zero,
{
    type Der1 = <V as Div<<G as Sub>::Output>>::Output;

    fn der1(&self, _: &G) -> Self::Der1 {
        Zero::zero()
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>> Func1dDer2<G> for PwConst1d<G, V>
where
    G: Clone + Sub<G>,
    V: Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Zero,
    <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output: Zero,
{
    type Der2 = <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output;

    fn der2(&self, _: &G) -> Self::Der2 {
        Zero::zero()
    }
}

impl<G: RelPos, V: Vector<<G as RelPos>::Output>> Func1dIntegrable<G> for PwConst1d<G, V>
where
    G: Clone + Sub,
    V: Mul<<G as Sub>::Output>,
    <V as Mul<<G as Sub>::Output>>::Output: Arithmetic,
{
    type Integrated = <V as Mul<<G as Sub>::Output>>::Output;

    fn integrate(&self, from: &G, to: &G) -> Self::Integrated {
        if to < from {
            return -self.integrate(to, from);
        }
        let lidx = self.knots.interval_index_of(from);
        let ridx = self.knots.interval_index_of(to);
        let one = <<G as RelPos>::Output as Scalar>::nearest_value_of(1.0);
        let w = <<G as RelPos>::Output as Scalar>::nearest_value_of(self.partition_ratio);

        // for the following case,
        // where f and t are from and to respectively and [i] is i-th knots
        //
        //      ---[0]---f---[1]-----[2]-----[3]---t---[4]---
        //
        // we will calculate the following 2 parts,
        //
        //      left_contrib  = [f ~ 1]
        //      right_contrib = [3 ~ t]
        //
        // and returns ([0 ~ 1] + [1 ~ 2] + [2 ~ 3] + [3 ~ 4]) - (left_contrib + right_contrib)
        //
        let mut res = Zero::zero();
        for i in lidx..ridx {
            let (gl, vl) = self.knots.force_get(i);
            let (gr, vr) = self.knots.force_get(i + 1);
            let weighted_v = (vl.clone() * &w) + (vr.clone() * &(one.clone() - &w));
            res += &(weighted_v * (gr.clone() - gl.clone()));
        }
        let left_trim = {
            let (gl, vl) = self.knots.force_get(lidx);
            let (gr, vr) = self.knots.force_get(lidx + 1);
            let point = from.relpos_between(gl, gr);
            // [l]---w---p---[r] => wl = w, wr = p - w
            // [1]---p---w---[r] => wl = p, wr = 0,
            let wl = (&point).partial_ord_min(&w).unwrap_or(&point);
            let wr = point.clone() - wl;
            let weighted_v = (vl.clone() * wl) + (vr.clone() * &wr);
            weighted_v * (gr.clone() - from.clone())
        };
        let right_trim = {
            let (gl, vl) = self.knots.force_get(ridx);
            let (gr, vr) = self.knots.force_get(ridx + 1);
            let point = to.relpos_between(gl, gr);
            // [l]---w---p---[r] => wl = 0, wr = 1 - p
            // [l]---p---w---[r] => wl = w - p, wr = 1 - w
            let wr = one.clone() - (&point).partial_ord_max(&w).unwrap_or(&point);
            let wl = one - &point - &wr;
            let weighted_v = (vl.clone() * &wl) + (vr.clone() * &wr);
            weighted_v * (to.clone() - gl.clone())
        };
        res -= &(left_trim + &right_trim);
        res
    }
}

// -----------------------------------------------------------------------------
// PwConst1dBuilder
//
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, JsonSchema))]
pub struct PwConst1dBuilder {
    /// Continuity of the interpolated function.
    continuity: SemiContinuity,
    /// Ratio determining partition point to use the left or right value. (0.0 <= partition <= 1.0)
    partition_ratio: f64,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for PwConst1dBuilder {
    fn deserialize<D>(deserializer: D) -> Result<PwConst1dBuilder, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data {
            continuity: SemiContinuity,
            partition_ratio: f64,
        }
        let data = Data::deserialize(deserializer)?;
        PwConst1dBuilder::new(data.continuity, data.partition_ratio)
            .map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl PwConst1dBuilder {
    /// Create a new `PwConst1dBuilder` instance.
    #[inline]
    pub fn new(cont: SemiContinuity, partition: f64) -> Result<Self, anyhow::Error> {
        if !(0. ..=1.).contains(&partition) {
            return Err(anyhow::anyhow!("partition must be in [0, 1]"));
        }
        Ok(Self {
            continuity: cont,
            partition_ratio: partition,
        })
    }
}

//
// methods
//
impl<G: RelPos, V: Vector<G::Output>> Interp1dBuilder<G, V> for PwConst1dBuilder
where
    G::Output: Into<f64>,
{
    type Output = PwConst1d<G, V>;
    type Err = anyhow::Error;

    fn build(self, gs: Vec<G>, vs: Vec<V>) -> Result<Self::Output, anyhow::Error> {
        PwConst1d::new(gs, vs, self.continuity, self.partition_ratio)
    }
}

impl<G: RelPos, V: Vector<G::Output>> DestructibleInterp1d for PwConst1d<G, V>
where
    G::Output: Into<f64>,
{
    type Builer = PwConst1dBuilder;

    fn destruct(self) -> (Self::Builer, Vec<G>, Vec<V>) {
        let (gs, vs) = self.knots.destruct();
        let builder =
            PwConst1dBuilder::new(self.continuity, self.partition_ratio).expect("valid builder");
        (builder, gs, vs)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::func1d::Func1d;

    use super::*;

    #[test]
    fn test_pwconst1d_new() {
        let grids = vec![0.0, 1.0, 2.0];
        let values = vec![0.0, 1.0, 0.0];
        let interp = super::PwConst1d::new(
            grids.clone(),
            values.clone(),
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(interp.knots().0, &grids);
        assert_eq!(interp.knots().1, &values);
        assert_eq!(interp.continuity, super::SemiContinuity::LeftContinuous);
        assert_eq!(interp.partition_ratio, 0.5);

        let interp = super::PwConst1d::new(
            grids.clone(),
            values.clone(),
            super::SemiContinuity::RightContinuous,
            0.0,
        );
        assert!(interp.is_ok());

        let interp = super::PwConst1d::new(
            grids.clone(),
            values.clone(),
            super::SemiContinuity::RightContinuous,
            1.0,
        );
        assert!(interp.is_ok());

        // errors
        assert!(super::PwConst1d::new(
            vec![0.0],
            vec![0.0],
            super::SemiContinuity::LeftContinuous,
            0.5
        )
        .is_err()); // too short

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0],
            Vec::<f64>::new(),
            super::SemiContinuity::LeftContinuous,
            0.5
        )
        .is_err()); // length mismatch

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0, 0.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5
        )
        .is_err()); // not sorted

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            -0.5
        )
        .is_err()); // invalid partition

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            -1e-10
        )
        .is_err()); // invalid partition

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            1.5
        )
        .is_err()); // invalid partition

        assert!(super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            1.0 + 1e-10
        )
        .is_err()); // invalid partition
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_pwconst1d_serialize() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        let serialized = serde_json::to_string(&interp).unwrap();
        assert_eq!(
            serialized,
            r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"left_continuous","partition_ratio":0.5}"#
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_pwconst1d_deserialize() {
        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"left_continuous","partition_ratio":0.5}"#;
        let deserialized: super::PwConst1d<f64, f64> = serde_json::from_str(serialized).unwrap();
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(deserialized, interp);

        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"right_continuous","partition_ratio":0.0}"#;
        let deserialized: super::PwConst1d<f64, f64> = serde_json::from_str(serialized).unwrap();
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::RightContinuous,
            0.0,
        )
        .expect("valid instance");
        assert_eq!(deserialized, interp);

        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"left_continuous","partition_ratio":1.0}"#;
        let deserialized: super::PwConst1d<f64, f64> = serde_json::from_str(serialized).unwrap();
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            1.0,
        )
        .expect("valid instance");
        assert_eq!(deserialized, interp);

        // errors
        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"left_continuous","partition_ratio":1.5}"#;
        let deserialized: Result<super::PwConst1d<f64, f64>, _> = serde_json::from_str(serialized);
        assert!(deserialized.is_err()); // invalid partition

        let serialized = r#"{"knots":[[0.0,0.0],[1.0,1.0],[2.0,0.0]],"continuity":"left_continuous","partition_ratio":-1.5}"#;
        let deserialized: Result<super::PwConst1d<f64, f64>, _> = serde_json::from_str(serialized);
        assert!(deserialized.is_err()); // invalid partition
    }

    #[test]
    fn test_pwconst1d_knots() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(interp.knots().0, &[0.0, 1.0, 2.0]);
        assert_eq!(interp.knots().1, &[0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_pwconst1d_interp() {
        // partition = 0.5
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(interp.interp(&-0.5), 0.0);
        assert_eq!(interp.interp(&0.0), 0.0);
        assert_eq!(interp.interp(&0.49999999), 0.0);
        assert_eq!(interp.interp(&0.5), 0.0);
        assert_eq!(interp.interp(&0.50000001), 1.0);
        assert_eq!(interp.interp(&1.0), 1.0);
        assert_eq!(interp.interp(&1.49999999), 1.0);
        assert_eq!(interp.interp(&1.5), 1.0);
        assert_eq!(interp.interp(&1.50000001), 0.0);
        assert_eq!(interp.interp(&2.0), 0.0);
        assert_eq!(interp.interp(&2.5), 0.0);

        // right continuous
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::RightContinuous,
            0.5,
        )
        .expect("valid instance");

        assert_eq!(interp.interp(&-0.5), 0.0);
        assert_eq!(interp.interp(&0.0), 0.0);
        assert_eq!(interp.interp(&0.49999999), 0.0);
        assert_eq!(interp.interp(&0.5), 1.0);
        assert_eq!(interp.interp(&0.50000001), 1.0);
        assert_eq!(interp.interp(&1.0), 1.0);
        assert_eq!(interp.interp(&1.49999999), 1.0);
        assert_eq!(interp.interp(&1.5), 0.0);
        assert_eq!(interp.interp(&1.50000001), 0.0);
        assert_eq!(interp.interp(&2.0), 0.0);
        assert_eq!(interp.interp(&2.5), 0.0);
    }

    #[test]
    fn test_pwconst1d_der1() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(interp.der1(&-0.5), 0.0);
        assert_eq!(interp.der1(&0.0), 0.0);
        assert_eq!(interp.der1(&0.5), 0.0);
        assert_eq!(interp.der1(&1.0), 0.0);
        assert_eq!(interp.der1(&1.5), 0.0);
        assert_eq!(interp.der1(&2.0), 0.0);
        assert_eq!(interp.der1(&2.5), 0.0);
    }

    #[test]
    fn test_pwconst1d_der2() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        assert_eq!(interp.der2(&-0.5), 0.0);
        assert_eq!(interp.der2(&0.0), 0.0);
        assert_eq!(interp.der2(&0.5), 0.0);
        assert_eq!(interp.der2(&1.0), 0.0);
        assert_eq!(interp.der2(&1.5), 0.0);
        assert_eq!(interp.der2(&2.0), 0.0);
        assert_eq!(interp.der2(&2.5), 0.0);
    }

    #[test]
    fn test_pwconst1d_der01() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");

        let (der0, der1) = interp.der01(&-0.5);
        assert_eq!(der0, interp.eval(&-0.5));
        assert_eq!(der1, interp.der1(&-0.5));

        let (der0, der1) = interp.der01(&0.0);
        assert_eq!(der0, interp.eval(&0.0));
        assert_eq!(der1, interp.der1(&0.0));

        let (der0, der1) = interp.der01(&0.5);
        assert_eq!(der0, interp.eval(&0.5));
        assert_eq!(der1, interp.der1(&0.5));

        let (der0, der1) = interp.der01(&1.0);
        assert_eq!(der0, interp.eval(&1.0));
        assert_eq!(der1, interp.der1(&1.0));

        let (der0, der1) = interp.der01(&1.5);
        assert_eq!(der0, interp.eval(&1.5));
        assert_eq!(der1, interp.der1(&1.5));
    }

    #[test]
    fn test_pwconst1d_der012() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");

        let (der0, der1, der2) = interp.der012(&-0.5);
        assert_eq!(der0, interp.eval(&-0.5));
        assert_eq!(der1, interp.der1(&-0.5));
        assert_eq!(der2, interp.der2(&-0.5));

        let (der0, der1, der2) = interp.der012(&0.0);
        assert_eq!(der0, interp.eval(&0.0));
        assert_eq!(der1, interp.der1(&0.0));
        assert_eq!(der2, interp.der2(&0.0));

        let (der0, der1, der2) = interp.der012(&0.5);
        assert_eq!(der0, interp.eval(&0.5));
        assert_eq!(der1, interp.der1(&0.5));
        assert_eq!(der2, interp.der2(&0.5));

        let (der0, der1, der2) = interp.der012(&1.0);
        assert_eq!(der0, interp.eval(&1.0));
        assert_eq!(der1, interp.der1(&1.0));
        assert_eq!(der2, interp.der2(&1.0));

        let (der0, der1, der2) = interp.der012(&1.5);
        assert_eq!(der0, interp.eval(&1.5));
        assert_eq!(der1, interp.der1(&1.5));
        assert_eq!(der2, interp.der2(&1.5));
    }

    #[test]
    fn test_pwconst1d_destruct() {
        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::LeftContinuous,
            0.5,
        )
        .expect("valid instance");
        let (builder, gs, vs) = interp.destruct();
        assert_eq!(builder.continuity, super::SemiContinuity::LeftContinuous);
        assert_eq!(builder.partition_ratio, 0.5);
        assert_eq!(gs, vec![0.0, 1.0, 2.0]);
        assert_eq!(vs, vec![0.0, 1.0, 0.0]);

        let interp = super::PwConst1d::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 0.0],
            super::SemiContinuity::RightContinuous,
            0.42,
        )
        .expect("valid instance");
        let (builder, gs, vs) = interp.destruct();
        assert_eq!(builder.continuity, super::SemiContinuity::RightContinuous);
        assert_eq!(builder.partition_ratio, 0.42);
        assert_eq!(gs, vec![0.0, 1.0, 2.0]);
        assert_eq!(vs, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_pwconst1dbuilder_new() {
        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 0.5);
        assert!(builder.is_ok());

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::RightContinuous, 0.42);
        assert!(builder.is_ok());

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 0.0);
        assert!(builder.is_ok());

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 1.0);
        assert!(builder.is_ok());

        let builder =
            super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 1.0 + 1e-10);
        assert!(builder.is_err());

        let builder =
            super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 0.0 - 1e-10);
        assert!(builder.is_err());

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, -0.5);
        assert!(builder.is_err());

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 1.5);
        assert!(builder.is_err());
    }

    #[test]
    fn test_pwconst1dbuilder_build() {
        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::LeftContinuous, 0.5)
            .expect("valid builder");
        let interp = builder
            .build(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
            .expect("valid instance");
        assert_eq!(interp.knots().0, &[0.0, 1.0, 2.0]);
        assert_eq!(interp.knots().1, &[0.0, 1.0, 0.0]);
        assert_eq!(interp.continuity, super::SemiContinuity::LeftContinuous);
        assert_eq!(interp.partition_ratio, 0.5);

        let builder = super::PwConst1dBuilder::new(super::SemiContinuity::RightContinuous, 0.42)
            .expect("valid builder");
        let interp = builder
            .build(vec![0.0, 1.0, 3.0], vec![0.0, 1.0, 5.0])
            .expect("valid instance");
        assert_eq!(interp.knots().0, &[0.0, 1.0, 3.0]);
        assert_eq!(interp.knots().1, &[0.0, 1.0, 5.0]);
        assert_eq!(interp.continuity, super::SemiContinuity::RightContinuous);
    }
}
