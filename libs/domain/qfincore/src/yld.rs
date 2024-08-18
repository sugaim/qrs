use std::fmt::Debug;

use qchrono::{duration::Duration, ext::chrono::Datelike};
use qmath::num::{Arithmetic, FloatBased, Real, Scalar};

use crate::daycount::{Act365f, StateLessYearFrac, YearFrac};

// -----------------------------------------------------------------------------
// Yield
// -----------------------------------------------------------------------------
/// A change ratio of a value over a year.
///
/// The dimension of this struct is 1/T, where T is a time unit.
/// Concrete unit of T is determined by the day count fraction and
/// we can recover the change ratio (not a percent nor a bps) between two dates
/// by multiplying the year fraction calculated with the given day count fraction.
///
/// # Example
/// ```
/// use qchrono::timepoint::Date;
/// use qfincore::{daycount::{YearFrac, Act360}, Yield};
///
/// let y = Yield {
///     day_count: Act360,
///     value: 0.02,
/// };
///
/// let stt: Date = "2021-01-01".parse().unwrap();
/// let end: Date = "2021-01-31".parse().unwrap();
///
/// let ratio = y.to_ratio(&stt, &end).unwrap();
/// assert_eq!(ratio, 0.02 * 30. / 360.);
/// ```
///
/// # Panics
///
/// Alghough this struct allows arithmetic operations,
/// we need to check that two [Yield] instances have the same day count fraction
/// to make the calculation consistent.
/// If this is not satisfied, the calculation will panic.
///
/// ```should_panic
/// use qfincore::{daycount::{Act360, Act365f, DayCount}, Yield};
///
/// let y1 = Yield {
///     day_count: DayCount::Act360,
///     value: 0.01,
/// };
/// let y2 = Yield {
///     day_count: DayCount::Act365f,
///     value: 0.02,
/// };
///
/// let _ = y1 + y2; // panics
/// ```
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Yield<Dcf, V> {
    pub day_count: Dcf,
    pub value: V,
}

//
// comp
//
impl<Dcf: Debug + Eq, V: PartialEq> PartialEq for Yield<Dcf, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(
            self.day_count, other.day_count,
            "Yields with different day count fractions are not comparable.",
        );
        self.value == other.value
    }
}

impl<Dcf: Debug + Eq, V: PartialOrd> PartialOrd for Yield<Dcf, V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        assert_eq!(
            self.day_count, other.day_count,
            "Yields with different day count fractions are not comparable.",
        );
        self.value.partial_cmp(&other.value)
    }
}

//
// methods
//
impl<Dcf, V> Yield<Dcf, V> {
    #[inline]
    pub fn convert<NewV>(self, f: impl FnOnce(V) -> NewV) -> Yield<Dcf, NewV> {
        Yield {
            value: f(self.value),
            day_count: self.day_count,
        }
    }

    #[inline]
    pub fn to_ratio<D: Datelike>(&self, stt: &D, end: &D) -> Result<V, Dcf::Error>
    where
        V: Real,
        Dcf: YearFrac<D>,
    {
        self.day_count
            .year_frac(stt, end)
            .map(|dcf| V::nearest_value_of_f64(dcf) * &self.value)
    }
}

impl<Dcf: Debug + Eq + StateLessYearFrac, V: Arithmetic> qmath::ext::num::Zero for Yield<Dcf, V> {
    #[inline]
    fn zero() -> Self {
        Self {
            value: V::zero(),
            day_count: Dcf::default(),
        }
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }
}
impl<Dcf, V: FloatBased> qmath::num::FloatBased for Yield<Dcf, V> {
    type BaseFloat = V::BaseFloat;
}
impl<Dcf, V: Arithmetic> std::ops::Neg for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            value: -self.value,
            day_count: self.day_count,
        }
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Add for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self + &rhs
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Add<&Self> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &Self) -> Self::Output {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        Self {
            value: self.value + &rhs.value,
            day_count: self.day_count,
        }
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::AddAssign<&Self> for Yield<Dcf, V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        self.value += &rhs.value;
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Sub for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self - &rhs
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Sub<&Self> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: &Self) -> Self::Output {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        Self {
            value: self.value - &rhs.value,
            day_count: self.day_count,
        }
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::SubAssign<&Self> for Yield<Dcf, V> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        self.value -= &rhs.value;
    }
}
impl<Dcf, V: Scalar> std::ops::Mul<&V::BaseFloat> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: &V::BaseFloat) -> Self::Output {
        Self {
            value: self.value * rhs,
            day_count: self.day_count,
        }
    }
}
impl<Dcf, V: Scalar> std::ops::MulAssign<&V::BaseFloat> for Yield<Dcf, V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &V::BaseFloat) {
        self.value *= rhs;
    }
}
impl<Dcf, V: Scalar> std::ops::Div<&V::BaseFloat> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: &V::BaseFloat) -> Self::Output {
        Self {
            value: self.value / rhs,
            day_count: self.day_count,
        }
    }
}
impl<Dcf, V: Scalar> std::ops::DivAssign<&V::BaseFloat> for Yield<Dcf, V> {
    #[inline]
    fn div_assign(&mut self, rhs: &V::BaseFloat) {
        self.value /= rhs;
    }
}

impl<V: Scalar> std::ops::Mul<Duration> for Yield<Act365f, V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> Self::Output {
        let dcf = rhs.approx_secs() / (24.0 * 60.0 * 60.0 * 365.0);
        self.value * &V::nearest_value_of_f64(dcf)
    }
}

#[cfg(test)]
mod tests {
    use qchrono::timepoint::DateTime;
    use qmath::ext::num::Zero;
    use rstest::rstest;

    use crate::daycount::DayCount;

    use super::*;

    #[test]
    fn test_convert() {
        let y = Yield {
            day_count: Act365f,
            value: 1i32,
        };

        let y = y.convert(|v| v as f64 * 0.5);

        assert_eq!(y.value, 0.5);
    }

    #[test]
    fn test_zero() {
        let y = Yield::<Act365f, f64>::zero();

        assert_eq!(y.day_count, Act365f);
        assert_eq!(y.value, 0f64);
    }

    #[test]
    fn test_is_zero() {
        let y = Yield::<Act365f, f64>::zero();

        assert!(y.is_zero());
    }
    #[rstest]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, 2.0)]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, -2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, 2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, -2.0)]
    fn test_cmp(
        #[case] dcf1: DayCount,
        #[case] value1: f64,
        #[case] dcf2: DayCount,
        #[case] value2: f64,
    ) {
        let y1 = Yield {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Yield {
            day_count: dcf2.clone(),
            value: value2,
        };

        let eq = y1 == y2;
        let cmp = y1.partial_cmp(&y2);

        assert_eq!(eq, value1 == value2);
        assert_eq!(cmp, value1.partial_cmp(&value2));
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_eq_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        let _ = y1 == y2;
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_cmp_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        let _ = y1.partial_cmp(&y2);
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0)]
    #[case(DayCount::Act365f, 0.0)]
    #[case(DayCount::Act365f, -3.0)]
    #[case(DayCount::Act360, 1.0)]
    #[case(DayCount::Act360, 0.0)]
    #[case(DayCount::Act360, -3.0)]
    fn test_neg(#[case] dcf: DayCount, #[case] value: f64) {
        let y = Yield {
            day_count: dcf.clone(),
            value,
        };

        let y = -y;

        assert_eq!(y.day_count, dcf);
        assert_eq!(y.value, -value);
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, 2.0)]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, -2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, 2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, -2.0)]
    fn test_add(
        #[case] dcf1: DayCount,
        #[case] value1: f64,
        #[case] dcf2: DayCount,
        #[case] value2: f64,
    ) {
        let y1 = Yield {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Yield {
            day_count: dcf2.clone(),
            value: value2,
        };

        let y = y1 + y2;

        assert_eq!(y.day_count, dcf1);
        assert_eq!(y.value, value1 + value2);
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_add_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        let _ = y1 + y2;
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, 2.0)]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, -2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, 2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, -2.0)]
    fn test_add_assign(
        #[case] dcf1: DayCount,
        #[case] value1: f64,
        #[case] dcf2: DayCount,
        #[case] value2: f64,
    ) {
        let mut y1 = Yield {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Yield {
            day_count: dcf2.clone(),
            value: value2,
        };

        y1 += &y2;

        assert_eq!(y1.day_count, dcf1);
        assert_eq!(y1.value, value1 + value2);
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_add_assign_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let mut y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        y1 += &y2;
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, 2.0)]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, -2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, 2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, -2.0)]
    fn test_sub(
        #[case] dcf1: DayCount,
        #[case] value1: f64,
        #[case] dcf2: DayCount,
        #[case] value2: f64,
    ) {
        let y1 = Yield {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Yield {
            day_count: dcf2.clone(),
            value: value2,
        };

        let y = y1 - y2;

        assert_eq!(y.day_count, dcf1);
        assert_eq!(y.value, value1 - value2);
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_sub_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        let _ = y1 - y2;
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, 2.0)]
    #[case(DayCount::Act365f, 1.0, DayCount::Act365f, -2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, 2.0)]
    #[case(DayCount::Act360, 1.0, DayCount::Act360, -2.0)]
    fn test_sub_assign(
        #[case] dcf1: DayCount,
        #[case] value1: f64,
        #[case] dcf2: DayCount,
        #[case] value2: f64,
    ) {
        let mut y1 = Yield {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Yield {
            day_count: dcf2.clone(),
            value: value2,
        };

        y1 -= &y2;

        assert_eq!(y1.day_count, dcf1);
        assert_eq!(y1.value, value1 - value2);
    }

    #[rstest]
    #[case(DayCount::Act365f, DayCount::Act360)]
    #[case(DayCount::Act360, DayCount::Act365f)]
    #[should_panic]
    fn test_sub_assign_panics(#[case] dcf1: DayCount, #[case] dcf2: DayCount) {
        let mut y1 = Yield {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Yield {
            day_count: dcf2,
            value: 2.0,
        };

        y1 -= &y2;
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, 2.0)]
    #[case(DayCount::Act365f, 1.0, -2.0)]
    #[case(DayCount::Act360, 1.0, 2.0)]
    #[case(DayCount::Act360, 1.0, -2.0)]
    fn test_mul(#[case] dcf: DayCount, #[case] value: f64, #[case] rhs: f64) {
        let y = Yield {
            day_count: dcf.clone(),
            value,
        };

        let y = y * &rhs;

        assert_eq!(y.day_count, dcf);
        assert_eq!(y.value, value * rhs);
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, 2.0)]
    #[case(DayCount::Act365f, 1.0, -2.0)]
    #[case(DayCount::Act360, 1.0, 2.0)]
    #[case(DayCount::Act360, 1.0, -2.0)]
    fn test_mul_assign(#[case] dcf: DayCount, #[case] value: f64, #[case] rhs: f64) {
        let mut y = Yield {
            day_count: dcf.clone(),
            value,
        };

        y *= &rhs;

        assert_eq!(y.day_count, dcf);
        assert_eq!(y.value, value * rhs);
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, 2.0)]
    #[case(DayCount::Act365f, 1.0, -2.0)]
    #[case(DayCount::Act360, 1.0, 2.0)]
    #[case(DayCount::Act360, 1.0, -2.0)]
    fn test_div(#[case] dcf: DayCount, #[case] value: f64, #[case] rhs: f64) {
        let y = Yield {
            day_count: dcf.clone(),
            value,
        };

        let y = y / &rhs;

        assert_eq!(y.day_count, dcf);
        assert_eq!(y.value, value / rhs);
    }

    #[rstest]
    #[case(DayCount::Act365f, 1.0, 2.0)]
    #[case(DayCount::Act365f, 1.0, -2.0)]
    #[case(DayCount::Act360, 1.0, 2.0)]
    #[case(DayCount::Act360, 1.0, -2.0)]
    fn test_div_assign(#[case] dcf: DayCount, #[case] value: f64, #[case] rhs: f64) {
        let mut y = Yield {
            day_count: dcf.clone(),
            value,
        };

        y /= &rhs;

        assert_eq!(y.day_count, dcf);
        assert_eq!(y.value, value / rhs);
    }

    #[rstest]
    #[case(0.02, "2021-01-01T00:00:00Z".parse().unwrap(), "2022-01-01T00:00:00Z".parse().unwrap())]
    #[case(0.05, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap())]
    #[case(0.01, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T12:00:00Z".parse().unwrap())]
    #[case(1.05, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T06:00:00Z".parse().unwrap())]
    #[case(1.0, "2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T03:00:00Z".parse().unwrap())]
    fn test_mul_duration(#[case] yld: f64, #[case] stt: DateTime, #[case] end: DateTime) {
        let y = Yield {
            day_count: Act365f,
            value: yld,
        };
        let year = Act365f.year_frac(&stt, &end).unwrap();

        let y = y * (end - stt);

        assert_eq!(y, yld * year);
    }
}
