use qmath::ext::num::Zero;

// -----------------------------------------------------------------------------
// Duration
// -----------------------------------------------------------------------------
/// Thin wrapper around [`chrono::Duration`].
///
/// Mainly used to override some operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    pub(crate) inner: chrono::Duration,
}

//
// conversion
//
impl From<Duration> for chrono::Duration {
    #[inline]
    fn from(inner: Duration) -> chrono::Duration {
        inner.inner
    }
}
impl From<chrono::Duration> for Duration {
    #[inline]
    fn from(inner: chrono::Duration) -> Self {
        Duration { inner }
    }
}

//
// ctors
//
impl Duration {
    #[inline]
    pub fn with_nanosecs(nanosecs: i64) -> Self {
        chrono::Duration::nanoseconds(nanosecs).into()
    }
    #[inline]
    pub fn with_microsecs(microsecs: i64) -> Self {
        chrono::Duration::microseconds(microsecs).into()
    }
    #[inline]
    pub fn with_millisecs(millisecs: i32) -> Self {
        chrono::Duration::milliseconds(millisecs.into()).into()
    }
    #[inline]
    pub fn try_with_millsecs(millisecs: i64) -> Option<Self> {
        chrono::Duration::try_milliseconds(millisecs).map(Into::into)
    }
    #[inline]
    pub fn with_secs(secs: i32) -> Self {
        chrono::Duration::seconds(secs.into()).into()
    }
    #[inline]
    pub fn try_with_secs(secs: i64) -> Option<Self> {
        chrono::Duration::try_seconds(secs).map(Into::into)
    }
    #[inline]
    pub fn with_mins(mins: i32) -> Self {
        chrono::Duration::minutes(mins.into()).into()
    }
    #[inline]
    pub fn try_with_mins(mins: i64) -> Option<Self> {
        chrono::Duration::try_minutes(mins).map(Into::into)
    }
    #[inline]
    pub fn with_hours(hours: i32) -> Self {
        chrono::Duration::hours(hours.into()).into()
    }
    #[inline]
    pub fn try_with_hours(hours: i64) -> Option<Self> {
        chrono::Duration::try_hours(hours).map(Into::into)
    }
    #[inline]
    pub fn with_days(days: i32) -> Self {
        chrono::Duration::days(days.into()).into()
    }
    #[inline]
    pub fn try_with_days(days: i64) -> Option<Self> {
        chrono::Duration::try_days(days).map(Into::into)
    }
}

//
// methods
//
impl Duration {
    /// Get the total number of seconds.
    ///
    /// We use [`f64`] as the result value to support fractional seconds.
    /// However, this value is not accurate because of the floating point error.
    ///
    /// # Example
    /// ```
    /// use qchrono::duration::Duration;
    ///
    /// let d = Duration::with_secs(1);
    /// assert_eq!(d.approx_secs(), 1.0);
    ///
    /// let d = Duration::with_secs(1) + Duration::with_microsecs(1);
    /// assert_eq!(d.approx_secs(), 1.000_001);
    ///
    /// let d = Duration::try_with_secs(10_000_000_000).unwrap() + Duration::with_nanosecs(1);
    /// assert_eq!(d.approx_secs(), 10_000_000_000.); // second term is ignored
    /// ```
    #[inline]
    pub fn approx_secs(&self) -> f64 {
        let sec = self.inner.num_seconds() as f64;
        let nano = self.inner.subsec_nanos() as f64 / 1_000_000_000.0;
        sec + nano
    }
}

//
// ops
//
impl std::ops::Neg for Duration {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        (-self.inner).into()
    }
}

impl std::ops::Add for Duration {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        (self.inner + rhs.inner).into()
    }
}

impl std::ops::Sub for Duration {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        (self.inner - rhs.inner).into()
    }
}

impl std::ops::Mul<i32> for Duration {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i32) -> Self {
        (self.inner * rhs).into()
    }
}

impl std::ops::Div<i32> for Duration {
    type Output = Self;

    #[inline]
    fn div(self, rhs: i32) -> Self {
        (self.inner / rhs).into()
    }
}

impl Zero for Duration {
    #[inline]
    fn zero() -> Self {
        chrono::Duration::zero().into()
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }
}

impl qmath::num::RelPos for Duration {
    type Output = f64;

    #[inline]
    fn relpos_between(&self, left: &Self, right: &Self) -> Option<f64> {
        let den = (*right - *left).approx_secs();
        if den == 0. {
            None
        } else {
            let num = (*self - *left).approx_secs();
            Some(num / den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test() {}

    #[rstest]
    #[case(Duration::zero(), 0.)]
    #[case(Duration::with_secs(1), 1.)]
    #[case(Duration::with_secs(-1), -1.)]
    #[case(Duration::with_mins(1), 60.)]
    #[case(Duration::with_hours(1), 3600.)]
    #[case(Duration::with_days(1), 86_400.)]
    #[case(Duration::with_nanosecs(1_000), 0.000_001)]
    #[case(Duration::with_nanosecs(1_234_567) + Duration::with_mins(3), 180.001_234_567)]
    #[case(Duration::with_nanosecs(1) + Duration::try_with_secs(10_000_000_000).unwrap(), 10_000_000_000.)]
    fn test_approx_secs(#[case] dur: Duration, #[case] expected: f64) {
        let tested = dur.approx_secs();

        assert_eq!(tested, expected);
    }
}
