use std::cmp::Ordering;

use anyhow::ensure;
use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};
use qmath::num::Real;

use crate::curve::YieldCurve;

use super::YieldCurveAdj;

// -----------------------------------------------------------------------------
// Bump
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, schemars::JsonSchema)]
pub struct Bump<V> {
    value: V,
    from: Option<DateTime>,
    to: Option<DateTime>,
}

//
// ser/de
//
impl<'de, V> serde::Deserialize<'de> for Bump<V>
where
    V: serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Data<V> {
            value: V,
            from: Option<DateTime>,
            to: Option<DateTime>,
        }
        let data = Data::deserialize(deserializer)?;
        Self::new(data.value, data.from, data.to).map_err(serde::de::Error::custom)
    }
}

//
// ctor
//
impl<V> Bump<V> {
    /// Create a new yield bump.
    ///
    /// This retuns [Result::Err] if the grid is empty.
    #[inline]
    pub fn new(value: V, from: Option<DateTime>, to: Option<DateTime>) -> anyhow::Result<Self> {
        if let (Some(from), Some(to)) = (&from, &to) {
            ensure!(from <= to, "Empty grid. from: {:?}, to: {:?}", from, to);
        }
        Ok(Bump { value, from, to })
    }
    #[inline]
    pub fn with_flat(value: V) -> Self {
        Self::new(value, None, None).unwrap()
    }
    #[inline]
    pub fn with_from(value: V, from: DateTime) -> Self {
        Self::new(value, Some(from), None).unwrap()
    }
    #[inline]
    pub fn with_to(value: V, to: DateTime) -> Self {
        Self::new(value, None, Some(to)).unwrap()
    }
}

impl<V, R: Real> YieldCurveAdj<R> for Bump<V>
where
    V: Clone + Into<R>,
{
    #[inline]
    fn adjusted_forward_rate<Y: YieldCurve<Value = R>>(
        &self,
        curve: &Y,
        f: &DateTime,
        t: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, R>> {
        match f.cmp(t) {
            Ordering::Greater => return self.adjusted_forward_rate(curve, t, f),
            Ordering::Equal => {
                let is_contained = self.from.as_ref().map(|stt| stt <= f).unwrap_or(true)
                    && self.to.as_ref().map(|end| t < end).unwrap_or(true);
                let base = curve.forward_rate(f, t)?;
                return Ok(if is_contained {
                    base.convert(|v| v + self.value.clone().into())
                } else {
                    base
                });
            }
            Ordering::Less => {}
        }
        let base = curve.forward_rate(f, t)?;
        let bump_stt = self.from.as_ref().map(|stt| stt.clamp(f, t)).unwrap_or(f);
        let bump_end = self.to.as_ref().map(|end| end.clamp(f, t)).unwrap_or(t);
        let weight = (bump_end - bump_stt).approx_secs() / (t - f).approx_secs();
        Ok(base.convert(|v| v + self.value.clone().into() * &R::nearest_value_of_f64(weight)))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;

    use crate::curve::atom::Flat;

    use super::*;

    #[test]
    fn test_new_ok() {
        let stt = DateTime::from_str("2021-01-01T00:00:00Z").unwrap();
        let end = DateTime::from_str("2021-01-02T00:00:00Z").unwrap();

        let bump = Bump::new(0.01, Some(stt), Some(end));

        assert!(bump.is_ok());
    }

    #[test]
    fn test_new_ng() {
        let stt = DateTime::from_str("2021-01-02T00:00:00Z").unwrap();
        let end = DateTime::from_str("2021-01-01T00:00:00Z").unwrap();

        let bump = Bump::new(0.01, Some(stt), Some(end));

        assert!(bump.is_err());
    }

    #[rstest]
    #[case::flat(None, None, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::flat(None, None, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.005)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-03T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::semi("2021-01-02T00:00:00Z".parse().ok(), None, "2021-01-03T00:00:00Z".parse().unwrap(), "2021-01-05T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.005)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::semi(None, "2021-01-02T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.0)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.005)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-04T00:00:00Z".parse().unwrap(), 0.005)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.01)]
    #[case::closed("2021-01-02T00:00:00Z".parse().ok(), "2021-01-03T00:00:00Z".parse().ok(), "2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.01)]
    fn test_adj(
        #[case] bump_stt: Option<DateTime>,
        #[case] bump_end: Option<DateTime>,
        #[case] stt: DateTime,
        #[case] end: DateTime,
        #[case] expected: f64,
    ) {
        let bump = Bump::new(0.01, bump_stt, bump_end).unwrap();
        let curve = Flat { rate: 0.05 };

        let res = bump.adjusted_forward_rate(&curve, &stt, &end).unwrap();
        let rev = bump.adjusted_forward_rate(&curve, &end, &stt).unwrap();

        approx::assert_abs_diff_eq!(res.value - 0.05, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(res.value, rev.value, epsilon = 1e-10);
    }
}
