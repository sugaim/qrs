use std::ops::{Div, Mul, Sub};

use qrs_collections::{MinSized, RequireMinSize, Series};

use crate::func1d::{Func1dDer1, Func1dDer2, Func1dIntegrable, SemiContinuity};
use crate::interp1d::{DestructibleInterp1d, Interp1d, Interp1dBuilder};
use crate::num::{Arithmetic, PartialOrdMinMax, RelPos, Scalar, Vector, Zero};

// -----------------------------------------------------------------------------
// PwConst1d
//

/// 1-dimensional linear interpolation.
///
/// # Example
/// ```
/// use qrs_collections::{RequireMinSize, Series};
/// use qrs_math::interp1d::Interp1d;
/// use qrs_math::func1d::SemiContinuity;
///
/// let grids = vec![0.0, 1.0, 2.0];
/// let values = vec![0.0, 1.0, 0.0];
/// let knots = Series::new(grids, values).unwrap().require_min_size().unwrap();
/// let cont = SemiContinuity::LeftContinuous;
/// let partition_ratio = 0.5;
/// let interp = qrs_math::interp1d::PwConst1d::new(knots, cont, partition_ratio).unwrap();
///
/// assert_eq!(interp.interp(&0.0), 0.0);
/// assert_eq!(interp.interp(&0.5), 0.0);
/// assert_eq!(interp.interp(&0.5001), 1.0);
/// assert_eq!(interp.interp(&1.0), 1.0);
/// assert_eq!(interp.interp(&1.5), 1.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, schemars::JsonSchema))]
pub struct PwConst1d<G, V> {
    /// Knots which are interpolated.
    #[cfg_attr(
        feature = "serde",
        serde(bound(serialize = "G: serde::Serialize + PartialOrd, V: serde::Serialize"))
    )]
    knots: MinSized<Series<G, V>, 2>,
    /// Continuity of the interpolated function.
    continuity: SemiContinuity,
    /// Ratio determining partition point to use the left or right value. (0.0 <= partition <= 1.0)
    partition_ratio: f64,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<'de, G, V> serde::Deserialize<'de> for PwConst1d<G, V>
where
    G: serde::Deserialize<'de> + PartialOrd,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<PwConst1d<G, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Data<G, V> {
            #[serde(bound(
                deserialize = "G: serde::Deserialize<'de> + PartialOrd, V: serde::Deserialize<'de>"
            ))]
            knots: MinSized<Series<G, V>, 2>,
            continuity: SemiContinuity,
            partition_ratio: f64,
        }
        let data = Data::deserialize(deserializer)?;
        Self::new(data.knots, data.continuity, data.partition_ratio)
            .map_err(serde::de::Error::custom)
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
        knots: MinSized<Series<G, V>, 2>,
        cont: SemiContinuity,
        partition: f64,
    ) -> Result<Self, anyhow::Error> {
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
        let idx = self.knots.interval_index_of(x).unwrap();
        let (gl, vl) = self.knots.get(idx).unwrap();
        let (gr, vr) = self.knots.get(idx + 1).unwrap();

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
        let lidx = self.knots.interval_index_of(from).unwrap();
        let ridx = self.knots.interval_index_of(to).unwrap();
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
            let (gl, vl) = self.knots.get(i).unwrap();
            let (gr, vr) = self.knots.get(i + 1).unwrap();
            let weighted_v = (vl.clone() * &w) + (vr.clone() * &(one.clone() - &w));
            res += &(weighted_v * (gr.clone() - gl.clone()));
        }
        let left_trim = {
            let (gl, vl) = self.knots.get(lidx).unwrap();
            let (gr, vr) = self.knots.get(lidx + 1).unwrap();
            let point = from.relpos_between(gl, gr);
            // [l]---w---p---[r] => wl = w, wr = p - w
            // [1]---p---w---[r] => wl = p, wr = 0,
            let wl = (&point).partial_ord_min(&w).unwrap_or(&point);
            let wr = point.clone() - wl;
            let weighted_v = (vl.clone() * wl) + (vr.clone() * &wr);
            weighted_v * (gr.clone() - from.clone())
        };
        let right_trim = {
            let (gl, vl) = self.knots.get(ridx).unwrap();
            let (gr, vr) = self.knots.get(ridx + 1).unwrap();
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, schemars::JsonSchema))]
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
impl<'de> serde::Deserialize<'de> for PwConst1dBuilder {
    fn deserialize<D>(deserializer: D) -> Result<PwConst1dBuilder, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
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

    #[inline]
    fn build(self, knots: Series<G, V>) -> Result<Self::Output, Self::Err> {
        PwConst1d::new(
            knots.require_min_size()?,
            self.continuity,
            self.partition_ratio,
        )
    }
}

impl<G: RelPos, V: Vector<G::Output>> DestructibleInterp1d for PwConst1d<G, V>
where
    G::Output: Into<f64>,
{
    type Builer = PwConst1dBuilder;

    fn destruct(self) -> (Self::Builer, Series<G, V>) {
        let builder =
            PwConst1dBuilder::new(self.continuity, self.partition_ratio).expect("valid builder");
        (builder, self.knots.into_inner())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use rstest::rstest;

    use crate::func1d::Func1d;

    use super::*;

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, false)]
    #[case(SemiContinuity::RightContinuous, 0.0, false)]
    #[case(SemiContinuity::RightContinuous, 1.0, false)]
    #[case(SemiContinuity::LeftContinuous, -0.5, true)]
    #[case(SemiContinuity::LeftContinuous, -1e-10, true)]
    #[case(SemiContinuity::LeftContinuous, 1.5, true)]
    #[case(SemiContinuity::LeftContinuous, 1.0 + 1e-10, true)]
    fn test_pwconst1d_new(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] is_err: bool,
    ) {
        let grids = vec![0.0, 1.0, 2.0];
        let values = vec![0.0, 1.0, 0.0];
        let knots = Series::new(grids.clone(), values.clone())
            .unwrap()
            .require_min_size()
            .unwrap();

        let interp = super::PwConst1d::new(knots, cont, partition);

        if is_err {
            assert!(interp.is_err());
        } else {
            let interp = interp.unwrap();
            assert_eq!(interp.continuity, cont);
            assert_eq!(interp.partition_ratio, partition);
            assert_eq!(interp.knots.grids(), &grids);
            assert_eq!(interp.knots.values(), &values);
        }
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, -0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.0, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.49999999, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.50000001, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.0, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.49999999, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.5, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.50000001, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.0, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.5, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, -0.5, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 0.0, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 0.49999999, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 0.5, 1.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 0.50000001, 1.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 1.0, 1.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 1.49999999, 1.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 1.5, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 1.50000001, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 2.0, 0.0)]
    #[case(SemiContinuity::RightContinuous, 0.5, 2.5, 0.0)]
    fn test_pwconst1d_interp(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] input: f64,
        #[case] expected: f64,
    ) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let res = interp.interp(&input);

        assert_abs_diff_eq!(res, expected, epsilon = 1e-10);
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, -0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.5)]
    fn test_pwconst1d_der1(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] input: f64,
    ) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let res = interp.der1(&input);

        assert_abs_diff_eq!(res, 0.0, epsilon = 1e-10);
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, -0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.5)]
    fn test_pwconst1d_der2(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] input: f64,
    ) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let res = interp.der2(&input);

        assert_abs_diff_eq!(res, 0.0, epsilon = 1e-10);
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, -0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.5)]
    fn test_pwconst1d_der01(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] input: f64,
    ) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let (der0, der1) = interp.der01(&input);

        assert_eq!(der0, interp.eval(&input));
        assert_eq!(der1, interp.der1(&input));
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, -0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 0.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 1.5)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.0)]
    #[case(SemiContinuity::LeftContinuous, 0.5, 2.5)]
    fn test_pwconst1d_der012(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] input: f64,
    ) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let (der0, der1, der2) = interp.der012(&input);

        assert_eq!(der0, interp.eval(&input));
        assert_eq!(der1, interp.der1(&input));
        assert_eq!(der2, interp.der2(&input));
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.24)]
    #[case(SemiContinuity::RightContinuous, 0.42)]
    fn test_pwconst1d_destruct(#[case] cont: SemiContinuity, #[case] partition: f64) {
        let interp = super::PwConst1d::new(
            Series::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
                .unwrap()
                .require_min_size()
                .unwrap(),
            cont,
            partition,
        )
        .unwrap();

        let (builder, knots) = interp.clone().destruct();

        assert_eq!(builder.continuity, cont);
        assert_eq!(builder.partition_ratio, partition);
        assert_eq!(knots.grids(), interp.knots.grids());
        assert_eq!(knots.values(), interp.knots.values());
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5, false)]
    #[case(SemiContinuity::RightContinuous, 0.0, false)]
    #[case(SemiContinuity::RightContinuous, 1.0, false)]
    #[case(SemiContinuity::LeftContinuous, -0.5, true)]
    #[case(SemiContinuity::LeftContinuous, -1e-10, true)]
    #[case(SemiContinuity::LeftContinuous, 1.5, true)]
    #[case(SemiContinuity::LeftContinuous, 1.0 + 1e-10, true)]
    fn test_pwconst1dbuilder_new(
        #[case] cont: SemiContinuity,
        #[case] partition: f64,
        #[case] is_err: bool,
    ) {
        let builder = super::PwConst1dBuilder::new(cont, partition);

        if is_err {
            assert!(builder.is_err());
        } else {
            let builder = builder.unwrap();
            assert_eq!(builder.continuity, cont);
            assert_eq!(builder.partition_ratio, partition);
        }
    }

    #[rstest]
    #[case(SemiContinuity::LeftContinuous, 0.5)]
    #[case(SemiContinuity::RightContinuous, 0.42)]
    fn test_pwconst1dbuilder_build(#[case] cont: SemiContinuity, #[case] partition: f64) {
        let builder = super::PwConst1dBuilder::new(cont, partition).expect("valid builder");
        let grids = vec![0.0, 1.0, 2.0];
        let values = vec![0.0, 1.0, 0.0];
        let knots = Series::new(grids.clone(), values.clone()).unwrap();

        let interp = builder.build(knots);

        assert!(interp.is_ok());
        let interp = interp.unwrap();
        assert_eq!(interp.continuity, cont);
        assert_eq!(interp.partition_ratio, partition);
        assert_eq!(interp.knots.grids(), &grids);
        assert_eq!(interp.knots.values(), &values);
    }
}
