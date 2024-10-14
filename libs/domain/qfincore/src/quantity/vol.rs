use std::fmt::Debug;

use anyhow::ensure;
use qchrono::ext::chrono::Datelike;
use qmath::num::{Arithmetic, FloatBased, Scalar};

use crate::daycount::{StateLessYearFrac, YearFrac};

// -----------------------------------------------------------------------------
// Volatility
// -----------------------------------------------------------------------------
/// A change ratio of a value over a square root of a year.
///
/// The dimension of this struct is 1/sqrt(T), where T is a time unit.
/// Concrete unit of T is determined by the day count fraction and
/// we can recover the change ratio (not a percent nor a bps) between two dates
/// by multiplying the year fraction calculated with the given day count fraction.
///
/// # Example
/// ```
/// use qchrono::timepoint::Date;
/// use qfincore::{daycount::{YearFrac, Act360}, quantity::Yield};
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
/// use qfincore::{daycount::{Act360, Act365f, DayCount}, quantity::Yield};
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
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Volatility<Dcf, V> {
    pub day_count: Dcf,
    pub value: V,
}

//
// comp
//
impl<Dcf: Debug + Eq, V: PartialEq> PartialEq for Volatility<Dcf, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(
            self.day_count, other.day_count,
            "Yields with different day count fractions are not comparable.",
        );
        self.value == other.value
    }
}

impl<Dcf: Debug + Eq, V: PartialOrd> PartialOrd for Volatility<Dcf, V> {
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
impl<Dcf, V> Volatility<Dcf, V> {
    #[inline]
    pub fn convert<NewV>(self, f: impl Fn(V) -> NewV) -> Volatility<Dcf, NewV> {
        Volatility {
            day_count: self.day_count,
            value: f(self.value),
        }
    }

    /// Calculate the change ratio between two dates.
    ///
    /// This returns an error if the year fraction is negative
    /// because we need to calculate the square root of the year fraction.
    #[inline]
    pub fn to_ratio<D>(&self, stt: &D, end: &D) -> anyhow::Result<V>
    where
        D: Datelike,
        V: Scalar,
        Dcf: YearFrac<D>,
        anyhow::Error: From<Dcf::Error>,
    {
        let dcf = self.day_count.year_frac(stt, end)?;
        ensure!(0. <= dcf, "year fraction must be non-negative");
        Ok(self.value.clone() * &V::nearest_value_of_f64(dcf.sqrt()))
    }
}

impl<Dcf: Debug + Eq + StateLessYearFrac, V: Arithmetic> qmath::ext::num::Zero
    for Volatility<Dcf, V>
{
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
impl<Dcf, V: FloatBased> qmath::num::FloatBased for Volatility<Dcf, V> {
    type BaseFloat = V::BaseFloat;
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Add for Volatility<Dcf, V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self + &rhs
    }
}
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::Add<&Self> for Volatility<Dcf, V> {
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
impl<Dcf: Debug + Eq, V: Arithmetic> std::ops::AddAssign<&Self> for Volatility<Dcf, V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        self.value += &rhs.value;
    }
}

#[cfg(test)]
mod tests {
    use qmath::ext::num::Zero;
    use rstest::rstest;

    use crate::daycount::{Act365f, DayCount};

    use super::*;

    #[test]
    fn test_convert() {
        let y = Volatility {
            day_count: Act365f,
            value: 1i32,
        };

        let y = y.convert(|v| v as f64 * 0.5);

        assert_eq!(y.value, 0.5);
    }

    #[test]
    fn test_zero() {
        let y = Volatility::<Act365f, f64>::zero();

        assert_eq!(y.day_count, Act365f);
        assert_eq!(y.value, 0f64);
    }

    #[test]
    fn test_is_zero() {
        let y = Volatility::<Act365f, f64>::zero();

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
        let y1 = Volatility {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Volatility {
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
        let y1 = Volatility {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Volatility {
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
        let y1 = Volatility {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Volatility {
            day_count: dcf2,
            value: 2.0,
        };

        let _ = y1.partial_cmp(&y2);
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
        let y1 = Volatility {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Volatility {
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
        let y1 = Volatility {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Volatility {
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
        let mut y1 = Volatility {
            day_count: dcf1.clone(),
            value: value1,
        };
        let y2 = Volatility {
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
        let mut y1 = Volatility {
            day_count: dcf1,
            value: 1.0,
        };
        let y2 = Volatility {
            day_count: dcf2,
            value: 2.0,
        };

        y1 += &y2;
    }
}
