use derivative::Derivative;

use qrs_chrono::DateTime;
use qrs_finance::core::daycount::Act365fRate;
use qrs_math::num::FloatBased;
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{YieldCurve, YieldCurveAdjust};

// -----------------------------------------------------------------------------
// Bump
//

/// Digital signal like bump on instant forward rates.
///
/// This can have a start and end datetime to limit the range of the bump.
///
/// # Example: Parallel bump
/// ```
/// use std::str::FromStr;
/// use approx::assert_abs_diff_eq;
/// use qrs_chrono::DateTime;
/// use qrs_finance::core::daycount::{Act365fRate, InterestRate};
///
/// use qrs_model::core::curve::{Bump, FlatCurve, YieldCurveAdjust};
///
/// let curve = FlatCurve { rate: Act365fRate::from_rate(0.01) };
/// let bump = Bump { delta: Act365fRate::from_rate(0.01), from: None, to: None };
/// let from = DateTime::from_str("2020-01-01T00:00:00Z").unwrap();
/// let to = DateTime::from_str("2020-01-02T00:00:00Z").unwrap();
/// let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
/// assert_abs_diff_eq!(res.into_value(), 0.02, epsilon = 1e-10);
/// ```
///
/// # Example: Grid bump
/// ```
/// use std::str::FromStr;
/// use approx::assert_abs_diff_eq;
/// use qrs_chrono::DateTime;
/// use qrs_finance::core::daycount::{Act365fRate, InterestRate};
///
/// use qrs_model::core::curve::{Bump, FlatCurve, YieldCurveAdjust};
///
/// let curve = FlatCurve { rate: Act365fRate::from_rate(0.01) };
/// let bump = Bump {
///     delta: Act365fRate::from_rate(0.01),
///     from: Some(DateTime::from_str("2020-01-02T00:00:00Z").unwrap()),
///     to: Some(DateTime::from_str("2020-01-04T00:00:00Z").unwrap())
/// };
/// let from = DateTime::from_str("2020-01-01T00:00:00Z").unwrap();
/// let to = DateTime::from_str("2020-01-05T00:00:00Z").unwrap();
/// let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
/// assert_abs_diff_eq!(res.into_value(), 0.015, epsilon = 1e-10);
/// ```
#[derive(Debug, Clone, Derivative)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, JsonSchema),
    serde(bound(
        serialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + Serialize",
        deserialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + Deserialize<'de>"
    ))
)]
#[derivative(PartialEq(
    bound = "V: PartialOrd + qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat>"
))]
pub struct Bump<V> {
    /// Bump size.
    pub delta: Act365fRate<V>,

    /// Start datetime of the bump. If it is not set, the bump is applied from the start of the curve.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub from: Option<DateTime>,

    /// End datetime of the bump. If it is not set, the bump is applied to the end of the curve.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub to: Option<DateTime>,
}

//
// methods
//
impl<C: YieldCurve> YieldCurveAdjust<C> for Bump<C::Value> {
    fn adjusted_forward_rate(
        &self,
        curve: &C,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<<C as YieldCurve>::Value>> {
        match from.cmp(to) {
            std::cmp::Ordering::Equal => return curve.forward_rate(from, to),
            std::cmp::Ordering::Greater => return self.adjusted_forward_rate(curve, to, from),
            _ => {}
        };
        let base = curve.forward_rate(from, to)?;
        let bump_from = match self.from {
            Some(ref dt) => dt.max(from),
            None => from,
        };
        let bump_to = match self.to {
            Some(ref dt) => dt.min(to),
            None => to,
        };
        // bump is applied on instant forward rate.
        // so we need a weight to adjust the bump.
        //    adjusted_forward_rate
        //      = \int_{from}^{to} [f(t) + bump(t)] dt / (to - from)
        //      = \int_{from}^{to} f(t) dt / (to - from) + \int_{from}^{to} bump(t) dt / (to - from)
        //      = forward_rate + bump * wegiht
        // where
        //    f(t): instant forward rate
        //    bump(t): step function like bump
        //    weight: defined by the following
        let weight = (bump_to - bump_from).millsecs() as f64 / (to - from).millsecs() as f64;
        let adj = self.delta.clone() * &<C::Value as FloatBased>::nearest_base_float_of(weight);
        Ok(base + adj)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use approx::assert_abs_diff_eq;
    use qrs_finance::core::daycount::InterestRate;

    use crate::core::curve::FlatCurve;

    use super::*;

    #[test]
    fn test_adjusted_forward_rate() {
        // parallel bump
        let curve = FlatCurve {
            rate: Act365fRate::from_rate(0.01),
        };
        let bump = Bump {
            delta: Act365fRate::from_rate(0.01),
            from: None,
            to: None,
        };
        let from = DateTime::from_str("2020-01-01T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-02T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.02, epsilon = 1e-10);

        // grid bump
        let curve = FlatCurve {
            rate: Act365fRate::from_rate(0.01),
        };
        let bump = Bump {
            delta: Act365fRate::from_rate(0.01),
            from: Some(DateTime::from_str("2020-01-02T00:00:00Z").unwrap()),
            to: Some(DateTime::from_str("2020-01-04T00:00:00Z").unwrap()),
        };

        let from = DateTime::from_str("2020-01-01T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-02T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.010, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-02T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-03T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.02, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-03T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-04T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.020, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-04T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-05T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.010, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-01T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-05T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.015, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-03T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-05T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.015, epsilon = 1e-10);

        let from = DateTime::from_str("2020-01-02T00:00:00Z").unwrap();
        let to = DateTime::from_str("2020-01-05T00:00:00Z").unwrap();
        let res = bump.adjusted_forward_rate(&curve, &from, &to).unwrap();
        assert_abs_diff_eq!(res.into_value(), 0.01 + 0.02 / 3., epsilon = 1e-10);
    }
}
