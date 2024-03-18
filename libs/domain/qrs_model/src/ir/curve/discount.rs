use std::{ops::Div, sync::Arc};

use derivative::Derivative;
use qrs_chrono::{DateTime, Duration, Velocity};
use qrs_datasrc::{CacheableSrc, DataSrc, DebugTree, TakeSnapshot};
use qrs_finance::{daycount::Act365fRate, products::core::Collateral, Ccy};
use qrs_math::num::{Real, RelPos};

use crate::core::curve::{AdjustedCurve, Curve, YieldCurve};

use super::IrCurveAdjust;

// -----------------------------------------------------------------------------
// DiscountKey
//
/// Key for discount curve
///
/// Discount curve is specified by currency and collateral.
/// `collateral` is [`None`] for uncollateralized products.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct DiscountKey {
    pub ccy: Ccy,
    pub collateral: Option<Collateral>,
}

// -----------------------------------------------------------------------------
// DiscountCurve
//
#[derive(Debug, Clone)]
pub struct DiscountCurve<V> {
    crv: AdjustedCurve<Arc<Curve<V>>, IrCurveAdjust<V>>,
}

//
// methods
//
impl<V> YieldCurve for DiscountCurve<V>
where
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        self.crv.forward_rate(from, to)
    }
}

impl<V: Real> DiscountCurve<V> {
    /// Returns the adjustments applied to the curve.
    /// Adjustments are applied in the order of the list.
    #[inline]
    pub fn adjustments(&self) -> &Vec<IrCurveAdjust<V>> {
        &self.crv.adjustments
    }

    /// Returns the unadjusted curve.
    #[inline]
    pub fn unadjusted(&self) -> &Curve<V> {
        &self.crv.base
    }

    /// Returns the weights of the components of the curve.
    ///
    /// Combining the result of this with adjustments,
    /// you can calculate impacts from bumping components of the curve.
    ///
    /// For example, the curve consists of components A, B, and C with weights 0.5, 0.3, and 0.2, respectively.
    /// If you bump this curve by 1bp, it is equivalent to bumping A by 0.5bp, B by 0.3bp, and C by 0.2bp.
    #[inline]
    pub fn weights(&self) -> impl ExactSizeIterator<Item = (&str, f64)> + Clone {
        self.crv
            .base
            .components
            .iter()
            .map(|(k, c)| (k.as_str(), c.weight))
    }
}

// -----------------------------------------------------------------------------
// DiscountReq
//
/// A request for a discount curve.
///
/// In addition to [`DiscountKey`], this request includes a list of adjustments to apply to the curve,
/// for example, to bump for IR-delta calculations, to shift for theta calculations, etc.
/// When multiple adjustments are specified, they are applied in the order of the list.
#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq(
    bound = "V: PartialOrd + qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat>"
))]
pub struct DiscountReq<V> {
    pub key: DiscountKey,
    pub adjustments: Vec<IrCurveAdjust<V>>,
}

// -----------------------------------------------------------------------------
// DiscountSrc
//
/// A source of discount curves.
///
/// Based on a source of curves, which returns a [`Curve`] for a given [`DiscountKey`].
/// This source is required to return a curve wrapped by [`Arc`] to reduce memory usage
/// and allow caching of the curves.
/// Please use [`DataSrc::map`] with [`Arc::new`] to wrap a curve by [`Arc`]
/// if the source returns a [`Curve`] directly.
#[derive(Debug, Clone, PartialEq, DebugTree)]
#[debug_tree(desc = "discount curve source")]
pub struct DiscountSrc<S> {
    #[debug_tree(subtree)]
    src: S,
}

//
// construction
//
impl<S> DiscountSrc<S> {
    #[inline]
    pub fn new(src: S) -> Self {
        Self { src }
    }
}

//
// methods
//
impl<S> DiscountSrc<S> {
    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &S {
        &self.src
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.src
    }

    /// Consumes this source and returns the inner source.
    #[inline]
    pub fn into_inner(self) -> S {
        self.src
    }
}

impl<S, V> DataSrc<DiscountKey> for DiscountSrc<S>
where
    S: DataSrc<DiscountKey, Output = Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    type Output = DiscountCurve<V>;

    #[inline]
    fn get(&self, key: &DiscountKey) -> anyhow::Result<Self::Output> {
        Ok(DiscountCurve {
            crv: AdjustedCurve {
                base: self.src.get(key)?,
                adjustments: vec![],
            },
        })
    }
}

impl<S, V> CacheableSrc<DiscountKey> for DiscountSrc<S>
where
    S: CacheableSrc<DiscountKey, Output = Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    #[inline]
    fn etag(&self, req: &DiscountKey) -> anyhow::Result<String> {
        self.src.etag(req)
    }
}

impl<S, V> DataSrc<DiscountReq<V>> for DiscountSrc<S>
where
    S: DataSrc<DiscountKey>,
    S::Output: Into<Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    type Output = DiscountCurve<V>;

    #[inline]
    fn get(&self, req: &DiscountReq<V>) -> anyhow::Result<Self::Output> {
        Ok(DiscountCurve {
            crv: AdjustedCurve {
                base: self.src.get(&req.key)?.into(),
                adjustments: req.adjustments.clone(),
            },
        })
    }
}

impl<S, V> CacheableSrc<DiscountReq<V>> for DiscountSrc<S>
where
    S: CacheableSrc<DiscountKey>,
    S::Output: Into<Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    #[inline]
    fn etag(&self, req: &DiscountReq<V>) -> anyhow::Result<String> {
        self.src.etag(&req.key)
    }
}

impl<S, V> TakeSnapshot<DiscountKey> for DiscountSrc<S>
where
    S: TakeSnapshot<DiscountKey, Output = Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    type Snapshot = DiscountSrc<S::Snapshot>;

    #[inline]
    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        DiscountKey: 'a,
        Rqs: IntoIterator<Item = &'a DiscountKey>,
    {
        Ok(DiscountSrc {
            src: self.src.take_snapshot(rqs)?,
        })
    }
}

impl<S, V> TakeSnapshot<DiscountReq<V>> for DiscountSrc<S>
where
    S: TakeSnapshot<DiscountKey>,
    S::Output: Into<Arc<Curve<V>>>,
    V: Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>>,
{
    type Snapshot = DiscountSrc<S::Snapshot>;

    #[inline]
    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        DiscountReq<V>: 'a,
        Rqs: IntoIterator<Item = &'a DiscountReq<V>>,
    {
        Ok(DiscountSrc {
            src: self.src.take_snapshot(rqs.into_iter().map(|r| &r.key))?,
        })
    }
}
