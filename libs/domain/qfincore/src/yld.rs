use std::fmt::Debug;

use qchrono::ext::chrono::Datelike;
use qmath::num::Real;

use crate::daycount::YearFrac;

// -----------------------------------------------------------------------------
// Yield
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Yield<Dcf, V> {
    pub day_count: Dcf,
    pub value: V,
}

//
// ctor
//
impl<Dcf: Default, V> From<V> for Yield<Dcf, V> {
    #[inline]
    fn from(value: V) -> Self {
        Yield {
            value,
            day_count: Dcf::default(),
        }
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

impl<Dcf: Debug + Eq + Default, V: Real> qmath::ext::num::Zero for Yield<Dcf, V> {
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
impl<Dcf, V: Real> qmath::num::FloatBased for Yield<Dcf, V> {
    type BaseFloat = V::BaseFloat;
}
impl<Dcf, V: Real> std::ops::Neg for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            value: -self.value,
            day_count: self.day_count,
        }
    }
}
impl<Dcf: Debug + Eq, V: Real> std::ops::Add for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        Self {
            value: self.value + rhs.value,
            day_count: self.day_count,
        }
    }
}
impl<Dcf: Debug + Eq, V: Real> std::ops::Add<&Self> for Yield<Dcf, V> {
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
impl<Dcf: Debug + Eq, V: Real> std::ops::AddAssign<&Self> for Yield<Dcf, V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        self.value += &rhs.value;
    }
}
impl<Dcf: Debug + Eq, V: Real> std::ops::Sub for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
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
impl<Dcf: Debug + Eq, V: Real> std::ops::Sub<&Self> for Yield<Dcf, V> {
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
impl<Dcf: Debug + Eq, V: Real> std::ops::SubAssign<&Self> for Yield<Dcf, V> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        assert_eq!(
            self.day_count, rhs.day_count,
            "day_count mismatch. This must be checked before."
        );
        self.value -= &rhs.value;
    }
}
impl<Dcf, V: Real> std::ops::Mul<&V::BaseFloat> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: &V::BaseFloat) -> Self::Output {
        Self {
            value: self.value * rhs,
            day_count: self.day_count,
        }
    }
}
impl<Dcf, V: Real> std::ops::MulAssign<&V::BaseFloat> for Yield<Dcf, V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &V::BaseFloat) {
        self.value *= rhs;
    }
}
impl<Dcf, V: Real> std::ops::Div<&V::BaseFloat> for Yield<Dcf, V> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: &V::BaseFloat) -> Self::Output {
        Self {
            value: self.value / rhs,
            day_count: self.day_count,
        }
    }
}
impl<Dcf, V: Real> std::ops::DivAssign<&V::BaseFloat> for Yield<Dcf, V> {
    #[inline]
    fn div_assign(&mut self, rhs: &V::BaseFloat) {
        self.value /= rhs;
    }
}
