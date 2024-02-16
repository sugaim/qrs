use std::ops::{Div, Sub};

use num::Zero;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    func1d::{Func1dDer1, Func1dDer2, SemiContinuity},
    num::{RelPos, Vector},
};

use super::{DestructibleInterp1d, Interp1d, Interp1dBuilder, _knots::Knots};

// -----------------------------------------------------------------------------
// PwConst1d
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
/// let interp = qrs_core::interp1d::PwConst1d::new(grids, values).unwrap();
///
/// assert_eq!(interp.interp(&-0.5), -0.5);
/// assert_eq!(interp.interp(&0.5), 0.5);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct PwConst1d<G, V> {
    /// Knots which are interpolated.
    #[serde(bound(serialize = "G: Serialize + PartialOrd, V: Serialize"))]
    knots: Knots<G, V>,
    /// Continuity of the interpolated function.
    continuity: SemiContinuity,
    /// Ratio determining partition point to use the left or right value. (0.0 <= partition <= 1.0)
    partition_ratio: f64,
}

//
// display, serde
//
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
        if partition < 0. || 1. < partition {
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
impl<G: RelPos, V: Vector<G::Output>> Interp1d for PwConst1d<G, V>
where
    G::Output: Into<f64>,
{
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

        let w: f64 = x.relpos_between(gl, gr).into();
        if w < self.partition_ratio {
            vl.clone()
        } else if self.partition_ratio < w {
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
    <G as RelPos>::Output: Into<f64>,
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
    <G as RelPos>::Output: Into<f64>,
    <V as Div<<G as Sub>::Output>>::Output: Div<<G as Sub>::Output>,
    <V as Div<<G as Sub>::Output>>::Output: Zero,
    <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output: Zero,
{
    type Der2 = <<V as Div<<G as Sub>::Output>>::Output as Div<<G as Sub>::Output>>::Output;

    fn der2(&self, _: &G) -> Self::Der2 {
        Zero::zero()
    }
}

// -----------------------------------------------------------------------------
// PwConst1dBuilder
//
#[derive(Clone, Copy, Debug, PartialEq, Serialize, JsonSchema)]
pub struct PwConst1dBuilder {
    /// Continuity of the interpolated function.
    continuity: SemiContinuity,
    /// Ratio determining partition point to use the left or right value. (0.0 <= partition <= 1.0)
    partition_ratio: f64,
}

//
// display, serde
//
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
        if partition < 0. || 1. < partition {
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
    use crate::{
        func1d::{Func1d, Func1dDer1, Func1dDer2},
        interp1d::{DestructibleInterp1d, Interp1d, Interp1dBuilder},
    };

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
