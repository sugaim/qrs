use std::{
    fmt::{Debug, Display},
    ops::Neg,
    str::FromStr,
};

use anyhow::{anyhow, bail, ensure};
use num::Zero;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize, Serializer};

use super::Velocity;

// -----------------------------------------------------------------------------
// Duration
//

/// Thin wrapper around `chrono::Duration` to override some traits.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    internal: chrono::Duration,
}

//
// display, serde
//
impl Debug for Duration {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for Duration {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut secs = self.internal.num_seconds();
        let mut sub_nanosecs = self.internal.subsec_nanos();

        let sign = if secs < 0 || sub_nanosecs < 0 {
            secs = -secs;
            sub_nanosecs = -sub_nanosecs;
            "-"
        } else {
            ""
        };

        let days = secs / (24 * 60 * 60);
        secs %= 24 * 60 * 60;
        let hours = secs / (60 * 60);
        secs %= 60 * 60;
        let mins = secs / 60;
        secs %= 60;

        if days != 0 {
            write!(f, "{}P{}D", sign, days)?;
        } else {
            write!(f, "{}P", sign)?;
        }
        if hours != 0 || mins != 0 || secs != 0 || sub_nanosecs != 0 {
            write!(f, "T")?;
        }
        if hours != 0 {
            write!(f, "{}H", hours)?;
        }
        if mins != 0 {
            write!(f, "{}M", mins)?;
        }
        if secs != 0 || sub_nanosecs != 0 {
            let secs = Decimal::new(secs, 0) + Decimal::new(sub_nanosecs as _, 9);
            write!(f, "{}S", secs)?;
        }
        Ok(())
    }
}

impl Serialize for Duration {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Duration {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl Zero for Duration {
    #[inline]
    fn zero() -> Self {
        chrono::Duration::zero().into()
    }
    #[inline]
    fn is_zero(&self) -> bool {
        self.internal.is_zero()
    }
}

impl Default for Duration {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl From<chrono::Duration> for Duration {
    #[inline]
    fn from(internal: chrono::Duration) -> Self {
        Self { internal }
    }
}

impl From<Duration> for chrono::Duration {
    #[inline]
    fn from(duration: Duration) -> Self {
        duration.internal
    }
}

impl FromStr for Duration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // parse global sign
        let (has_negative_sign, s) = match s.chars().next() {
            Some('-') => (true, &s[1..]),
            Some('+') => (false, &s[1..]),
            _ => (false, s),
        };

        // check 'P' prefix
        ensure!(
            s.starts_with('P'),
            "Invalid duration format '{s}'. Duration must start with 'P'."
        );
        let s = &s[1..];

        // split with respect to 'T'
        ensure!(
            s.chars().filter(|&c| c == 'T').count() <= 1,
            "Invalid duration format '{s}'. Duration must not contain more than one 'T'."
        );
        let (s_date, s_time) = match s.split_once('T') {
            Some((date, time)) => (date, Some(time)),
            None => (s, None),
        };
        if s_date.is_empty() && s_time.is_none() {
            bail!("Invalid duration format '{s}'. Duration must contain at least one of date or time part.");
        }

        // parse date part
        let days = _parse_dur_date_part(s_date)?;

        // parse time part
        let (hours, mins, secs) = match s_time {
            Some(s_time) => {
                let (hours, s_time) = _parse_dur_hour(s_time)?;
                let (mins, s_time) = _parse_dur_min(s_time)?;
                let (secs, s_time) = _parse_dur_sec(s_time)?;
                ensure!(
                    s_time.is_empty(),
                    "Invalid duration format '{s}'. Time part must not contain any characters after 'S'."
                );
                (hours, mins, secs)
            }
            None => (Duration::zero(), Duration::zero(), Duration::zero()),
        };

        let duration = days + hours + mins + secs;
        Ok(if has_negative_sign {
            -duration
        } else {
            duration
        })
    }
}

fn _parse_dur_date_part(s_date: &str) -> Result<Duration, anyhow::Error> {
    let gen_err = || {
        anyhow::anyhow!(
            "Invalid duration format. Date part must be '[integer]D' or '[integer]W'. \
            Note that 'Y' and 'M' are not supported because these are ambiguous. \
            Date part was '{s_date}'"
        )
    };

    if s_date.is_empty() {
        return Ok(Duration::zero());
    }
    if !s_date.ends_with('D') && !s_date.ends_with('W') {
        bail!(gen_err());
    }
    let unit = if s_date.ends_with('D') { 1 } else { 7 };
    let s_count = &s_date[..s_date.len() - 1];
    let (sign, s_date) = match s_count.chars().next() {
        Some('-') => (-1, &s_count[1..]),
        Some('+') => (1, &s_count[1..]),
        _ => (1, s_count),
    };
    let count = s_date.parse::<i64>().map_err(|_| gen_err())?;
    Ok(Duration::with_days(sign * count * unit))
}

fn _parse_signed<T>(s: &str) -> Result<T, T::Err>
where
    T: FromStr + Neg<Output = T>,
{
    let (is_neg, s) = match s.chars().next() {
        Some('-') => (true, &s[1..]),
        Some('+') => (false, &s[1..]),
        _ => (false, s),
    };
    let value = s.parse::<T>()?;
    Ok(if is_neg { -value } else { value })
}

fn _parse_dur_hour(s_time: &str) -> Result<(Duration, &str), anyhow::Error> {
    let (s_hour, s_rem) = match s_time.split_once('H') {
        Some((hour, rem)) => (hour, rem),
        None => return Ok((Duration::zero(), s_time)),
    };
    ensure!(
        s_rem.chars().filter(|&c| c == 'H').count() == 0,
        "Invalid duration format. Time part must not contain more than one 'H'."
    );
    Ok((
        _parse_signed(s_hour).map_err(|_| {
            anyhow!(
                "Invalid duration format. Hour must appear at the first of time part and in '[integer]H' format. \
                Recognized hour part was '{s_hour}H'"
            )
        }).map(Duration::with_hours)?,
        s_rem,
    ))
}
fn _parse_dur_min(s_time: &str) -> Result<(Duration, &str), anyhow::Error> {
    let (s_min, s_rem) = match s_time.split_once('M') {
        Some((min, rem)) => (min, rem),
        None => return Ok((Duration::zero(), s_time)),
    };
    ensure!(
        s_rem.chars().filter(|&c| c == 'M').count() == 0,
        "Invalid duration format. Time part must not contain more than one 'M'."
    );
    Ok((
        _parse_signed(s_min).map_err(|_| {
            anyhow!(
                "Invalid duration format. Minute must appear at the first of time part and in '[integer]M' format. \
                Recognized minute part was '{s_min}M'"
            )
        })
        .map(Duration::with_mins)?,
        s_rem,
    ))
}
fn _parse_dur_sec(s_time: &str) -> Result<(Duration, &str), anyhow::Error> {
    let (s_sec, s_rem) = match s_time.split_once('S') {
        Some((sec, rem)) => (sec, rem),
        None => return Ok((Duration::zero(), s_time)),
    };
    ensure!(
        s_rem.chars().filter(|&c| c == 'S').count() == 0,
        "Invalid duration format. Time part must not contain more than one 'S'."
    );
    let (secs, sub_nanosecs) = {
        let dec_secs: Decimal = _parse_signed(s_sec).map_err(|_| {
            anyhow!(
                "Invalid duration format. Second must appear at the first of time part and in '[integer or float]S' format. \
                Recognized second part was '{s_sec}S'"
            )
        })?;
        let mantissa = dec_secs.mantissa();
        let scale = 10i128.pow(dec_secs.scale());

        let sub_nanosecs = if dec_secs.scale() <= 9 {
            (mantissa % scale) * 10i128.pow(9 - dec_secs.scale())
        } else {
            (mantissa % scale) / 10i128.pow(dec_secs.scale() - 9)
        };
        (mantissa / scale, sub_nanosecs)
    };
    Ok((
        Duration::with_nanosecs((secs * 1_000_000_000 + sub_nanosecs) as i64),
        s_rem,
    ))
}

impl Duration {
    /// Same as [chrono::Duration::zero].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_days(count as i64);
    ///    let chrono_obj = chrono::Duration::days(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_days(days: i64) -> Self {
        chrono::Duration::days(days).into()
    }

    /// Same as [chrono::Duration::weeks].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_weeks(count as i64);
    ///    let chrono_obj = chrono::Duration::weeks(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_weeks(days: i64) -> Self {
        chrono::Duration::weeks(days).into()
    }

    /// Same as [chrono::Duration::hours].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_hours(count as i64);
    ///    let chrono_obj = chrono::Duration::hours(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_hours(hours: i64) -> Self {
        chrono::Duration::hours(hours).into()
    }

    /// Same as [chrono::Duration::minutes].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_mins(count as i64);
    ///    let chrono_obj = chrono::Duration::minutes(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_mins(mins: i64) -> Self {
        chrono::Duration::minutes(mins).into()
    }

    /// Same as [chrono::Duration::seconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_secs(count as i64);
    ///    let chrono_obj = chrono::Duration::seconds(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_secs(seconds: i64) -> Self {
        chrono::Duration::seconds(seconds).into()
    }

    /// Same as [chrono::Duration::milliseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_millsecs(count as i64);
    ///    let chrono_obj = chrono::Duration::milliseconds(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_millsecs(millsecs: i64) -> Self {
        chrono::Duration::milliseconds(millsecs).into()
    }

    /// Same as [chrono::Duration::microseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_microsecs(count as i64);
    ///    let chrono_obj = chrono::Duration::microseconds(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_microsecs(microsecs: i64) -> Self {
        chrono::Duration::microseconds(microsecs).into()
    }

    /// Same as [chrono::Duration::nanoseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i32::MIN, -42, -1, 0, 1, 42, i32::MAX] {
    ///    let qcore_obj = Duration::with_nanosecs(count as i64);
    ///    let chrono_obj = chrono::Duration::nanoseconds(count as i64);
    ///    assert_eq!(qcore_obj, Duration::from(chrono_obj));
    /// }
    /// ```
    #[inline]
    pub fn with_nanosecs(nannosecs: i64) -> Self {
        chrono::Duration::nanoseconds(nannosecs).into()
    }
}

//
// getters
//
impl Duration {
    /// Returns the internal [chrono::Duration] reference.
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///
    ///     assert_eq!(qcore_obj.as_chrono(), &chrono_obj);
    /// }
    /// ```
    #[inline]
    pub fn as_chrono(&self) -> &chrono::Duration {
        &self.internal
    }

    /// Same as [chrono::Duration::num_days].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///
    ///     assert_eq!(qcore_obj.days(), chrono_obj.num_days());
    /// }
    /// ```
    #[inline]
    pub fn days(&self) -> i64 {
        self.internal.num_days()
    }

    /// Same as [chrono::Duration::num_hours].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.hours(), chrono_obj.num_hours());
    /// }
    /// ```
    #[inline]
    pub fn hours(&self) -> i64 {
        self.internal.num_hours()
    }

    /// Same as [chrono::Duration::num_minutes].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.mins(), chrono_obj.num_minutes());
    /// }
    /// ```
    #[inline]
    pub fn mins(&self) -> i64 {
        self.internal.num_minutes()
    }

    /// Same as [chrono::Duration::num_seconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.secs(), chrono_obj.num_seconds());
    /// }
    /// ```
    #[inline]
    pub fn secs(&self) -> i64 {
        self.internal.num_seconds()
    }

    /// Same as [chrono::Duration::num_milliseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.millsecs(), chrono_obj.num_milliseconds());
    /// }
    /// ```
    #[inline]
    pub fn millsecs(&self) -> i64 {
        self.internal.num_milliseconds()
    }

    /// Same as [chrono::Duration::num_microseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.microsecs(), chrono_obj.num_microseconds());
    /// }
    /// ```
    #[inline]
    pub fn microsecs(&self) -> Option<i64> {
        self.internal.num_microseconds()
    }

    /// Same as [chrono::Duration::num_nanoseconds].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.nanosecs(), chrono_obj.num_nanoseconds());
    /// }
    /// ```
    #[inline]
    pub fn nanosecs(&self) -> Option<i64> {
        self.internal.num_nanoseconds()
    }

    /// Same as [chrono::Duration::subsec_nanos].
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// for count in [i64::MIN, -42, -1, 0, 1, 42, i64::MAX] {
    ///     let chrono_obj = chrono::Duration::nanoseconds(count);
    ///     let qcore_obj = Duration::from(chrono_obj);
    ///     assert_eq!(qcore_obj.subsec_nanos(), chrono_obj.subsec_nanos());
    /// }
    /// ```
    #[inline]
    pub fn subsec_nanos(&self) -> i32 {
        self.internal.subsec_nanos()
    }
}

//
// operators
//
impl std::ops::Neg for Duration {
    type Output = Duration;

    #[inline]
    fn neg(self) -> Self::Output {
        (-self.internal).into()
    }
}

macro_rules! define_self_self_op {
    ($trait:ident, $fn:ident) => {
        impl std::ops::$trait for Duration {
            type Output = Duration;

            #[inline]
            fn $fn(self, rhs: Self) -> Self::Output {
                self.internal.$fn(rhs.internal).into()
            }
        }
        impl std::ops::$trait<&Duration> for Duration {
            type Output = Duration;

            #[inline]
            fn $fn(self, rhs: &Self) -> Self::Output {
                self.internal.$fn(rhs.internal).into()
            }
        }
    };
}

define_self_self_op!(Add, add);
define_self_self_op!(Sub, sub);

macro_rules! define_self_self_assign {
    ($trait:ident, $fn:ident) => {
        impl std::ops::$trait for Duration {
            #[inline]
            fn $fn(&mut self, rhs: Self) {
                self.internal.$fn(rhs.internal);
            }
        }
        impl std::ops::$trait<&Duration> for Duration {
            #[inline]
            fn $fn(&mut self, rhs: &Self) {
                self.internal.$fn(rhs.internal);
            }
        }
    };
}

define_self_self_assign!(AddAssign, add_assign);
define_self_self_assign!(SubAssign, sub_assign);

macro_rules! define_self_int_op {
    ($trait:ident, $int:ty, $fn:ident) => {
        impl std::ops::$trait<$int> for Duration {
            type Output = Duration;

            #[inline]
            fn $fn(self, rhs: $int) -> Self::Output {
                self.internal.$fn(rhs as i32).into()
            }
        }
        impl std::ops::$trait<&$int> for Duration {
            type Output = Duration;

            #[inline]
            fn $fn(self, rhs: &$int) -> Self::Output {
                self.internal.$fn(*rhs as i32).into()
            }
        }
    };
}

define_self_int_op!(Mul, i32, mul);
define_self_int_op!(Div, i32, div);

macro_rules! define_datetime_self_op {
    ($trait:ident, $fn:ident) => {
        impl<Tz: chrono::TimeZone> std::ops::$trait<Duration> for chrono::DateTime<Tz> {
            type Output = chrono::DateTime<Tz>;

            #[inline]
            fn $fn(self, rhs: Duration) -> Self::Output {
                self.$fn(rhs.internal)
            }
        }
        impl<Tz: chrono::TimeZone> std::ops::$trait<&Duration> for chrono::DateTime<Tz> {
            type Output = chrono::DateTime<Tz>;

            #[inline]
            fn $fn(self, rhs: &Duration) -> Self::Output {
                self.$fn(rhs.internal)
            }
        }
    };
}

define_datetime_self_op!(Add, add);
define_datetime_self_op!(Sub, sub);

macro_rules! define_datetime_self_assign {
    ($trait:ident, $fn:ident) => {
        impl<Tz: chrono::TimeZone> std::ops::$trait<Duration> for chrono::DateTime<Tz> {
            #[inline]
            fn $fn(&mut self, rhs: Duration) {
                self.$fn(rhs.internal);
            }
        }
        impl<Tz: chrono::TimeZone> std::ops::$trait<&Duration> for chrono::DateTime<Tz> {
            #[inline]
            fn $fn(&mut self, rhs: &Duration) {
                self.$fn(rhs.internal);
            }
        }
    };
}

define_datetime_self_assign!(AddAssign, add_assign);
define_datetime_self_assign!(SubAssign, sub_assign);

// div
macro_rules! define_div_to_velocity {
    ($t:ty) => {
        impl std::ops::Div<Duration> for $t {
            type Output = Velocity<$t>;

            #[inline]
            fn div(self, rhs: Duration) -> Self::Output {
                Velocity::new(self, rhs)
            }
        }
        impl std::ops::Div<&Duration> for $t {
            type Output = Velocity<$t>;

            #[inline]
            fn div(self, rhs: &Duration) -> Self::Output {
                Velocity::new(self, *rhs)
            }
        }
    };
}

define_div_to_velocity!(f32);
define_div_to_velocity!(f64);

// =============================================================================
#[cfg(test)]
mod tests {
    use rstest::rstest;
    use rstest_reuse::{self, *};

    use super::*;

    #[template]
    #[rstest]
    #[case::d("P1D", Some(Duration::with_days(1)))]
    #[case::d("P+1D", Some(Duration::with_days(1)))]
    #[case::d("P-2D", Some(-Duration::with_days(2)))]
    #[case::w("P1W", Some(Duration::with_days(7)))]
    #[case::w("P+1W", Some(Duration::with_days(7)))]
    #[case::w("P-2W", Some(-Duration::with_days(14)))]
    #[case::h("PT1H", Some(Duration::with_hours(1)))]
    #[case::h("PT+1H", Some(Duration::with_hours(1)))]
    #[case::h("PT-2H", Some(-Duration::with_hours(2)))]
    #[case::m("PT1M", Some(Duration::with_mins(1)))]
    #[case::m("PT+1M", Some(Duration::with_mins(1)))]
    #[case::m("PT-2M", Some(-Duration::with_mins(2)))]
    #[case::s("PT1S", Some(Duration::with_secs(1)))]
    #[case::s("PT+1S", Some(Duration::with_secs(1)))]
    #[case::s("PT-2S", Some(-Duration::with_secs(2)))]
    #[case::s("PT1.5S", Some(Duration::with_nanosecs(1_500_000_000)))]
    #[case::s("PT-2.5S", Some(-Duration::with_nanosecs(2_500_000_000)))]
    #[case::s("PT0.000000042S", Some(Duration::with_nanosecs(42)))]
    #[case::s("PT-0.000000042S", Some(Duration::with_nanosecs(-42)))]
    #[case::mix("PT1H1M1S", Some(Duration::with_hours(1) + Duration::with_mins(1) + Duration::with_secs(1)))]
    #[case::mix("PT2H-2M-2S", Some(Duration::with_hours(2) - Duration::with_mins(2) - Duration::with_secs(2)))]
    #[case::mix("PT1H1M1.5S", Some(Duration::with_hours(1) + Duration::with_mins(1) + Duration::with_nanosecs(1_500_000_000)))]
    #[case::mix("PT-2H2M-2.5S", Some(-Duration::with_hours(2) + Duration::with_mins(2) - Duration::with_nanosecs(2_500_000_000)))]
    #[case::mix("P1DT1H1M1S", Some(Duration::with_days(1) + Duration::with_hours(1) + Duration::with_mins(1) + Duration::with_secs(1)))]
    #[case::mix("P-2DT-2H-2M2S", Some(-Duration::with_days(2) - Duration::with_hours(2) - Duration::with_mins(2) + Duration::with_secs(2)))]
    #[case::mix("P1DT1H1M1.5S", Some(Duration::with_days(1) + Duration::with_hours(1) + Duration::with_mins(1) + Duration::with_nanosecs(1_500_000_000)))]
    #[case::mix("P-2DT-2H-2M-2.5S", Some(-Duration::with_days(2) - Duration::with_hours(2) - Duration::with_mins(2) - Duration::with_nanosecs(2_500_000_000)))]
    #[case::invalid("", None)] // empty
    #[case::invalid("P", None)] // no date or time part
    #[case::invalid("P1", None)] // no date or time part
    #[case::invalid("PT3HT4S", None)] // multiple time part
    #[case::invalid("P1D1D", None)] // multiple date part
    #[case::invalid("P1W1W", None)] // multiple date part
    #[case::invalid("P1D1W", None)] // multiple date part
    #[case::invalid("P1H1H", None)] // multiple time part
    #[case::invalid("P1M1M", None)] // multiple time part
    #[case::invalid("P1S1S", None)] // multiple time part
    #[case::invalid("P1H", None)] // no time specifier 'T'
    #[case::invalid("P1S", None)] // no time specifier 'T'
    #[case::invalid(" P1D", None)] // leading space
    #[case::invalid("P1D ", None)] // trailing space
    #[case::invalid("P 1D", None)] // space in the middle
    #[case::invalid("P1D T1H", None)] // space in the middle
    #[case::invalid("P1Y", None)] // year is not supported
    #[case::invalid("P1M", None)] // month is not supported
    #[case::invalid("P1DT1Y", None)] // year is not supported
    #[case::invalid("P1DT1Y1M", None)] // year is not supported
    #[case::invalid("P1DT1Y1M1H", None)] // year is not supported
    #[case::invalid("P1DT1Y1M1H1S", None)] // year is not supported
    #[case::invalid("PxD", None)] // invalid characters
    #[case::invalid("PTxH", None)] // invalid characters
    #[case::invalid("PTxM", None)] // invalid characters
    #[case::invalid("PTxS", None)] // invalid characters
    fn cases_for_parse(#[case] s: &str, #[case] expected: Option<Duration>) {}

    #[template]
    #[rstest]
    #[case(Duration::zero())]
    #[case(Duration::with_days(1))]
    #[case(Duration::with_days(2))]
    #[case(Duration::with_hours(1))]
    #[case(Duration::with_hours(2))]
    #[case(Duration::with_mins(1))]
    #[case(Duration::with_mins(2))]
    #[case(Duration::with_secs(1))]
    #[case(Duration::with_secs(2))]
    #[case(Duration::with_millsecs(1))]
    #[case(Duration::with_millsecs(2))]
    #[case(Duration::with_microsecs(1))]
    #[case(Duration::with_microsecs(2))]
    #[case(Duration::with_nanosecs(1))]
    #[case(Duration::with_nanosecs(2))]
    #[case(Duration::with_secs(1) + Duration::with_nanosecs(1011))]
    fn cases_for_display(#[case] d: Duration) {}

    #[test]
    fn test_default() {
        assert_eq!(Duration::default(), Duration::zero());
    }

    #[test]
    fn test_conversion() {
        // chrono -> qrs
        let chrono_obj = chrono::Duration::days(1);
        let qcore_obj = Duration::from(chrono_obj);
        assert_eq!(qcore_obj, Duration::with_days(1));

        // qrs -> chrono
        let qcore_obj = Duration::with_days(1);
        let chrono_obj: chrono::Duration = qcore_obj.into();
        assert_eq!(chrono_obj, chrono::Duration::days(1));
    }

    #[test]
    fn test_debug() {
        let d = Duration::with_days(1);
        assert_eq!(format!("{:?}", d), "P1D");
        assert_eq!(format!("{:?}", -d), "-P1D");

        let d = Duration::with_hours(1);
        assert_eq!(format!("{:?}", d), "PT1H");
        assert_eq!(format!("{:?}", -d), "-PT1H");

        let d = Duration::with_nanosecs(1);
        assert_eq!(format!("{:?}", d), "PT0.000000001S");
        assert_eq!(format!("{:?}", -d), "-PT0.000000001S");
    }

    #[apply(cases_for_parse)]
    fn test_from_str(s: &str, expected: Option<Duration>) {
        // cases
        let cases = vec![
            (s.to_owned(), expected),
            (format!("+{}", s), expected),
            (format!("-{}", s), expected.map(|d| -d)),
        ];

        for (s, expected) in cases {
            let actual: Result<Duration, _> = s.parse();
            match expected {
                Some(expected) => {
                    assert!(actual.is_ok());
                    assert_eq!(actual.unwrap(), expected);
                }
                None => {
                    assert!(actual.is_err());
                }
            }
        }
    }

    #[apply(cases_for_display)]
    fn test_display(d: Duration) {
        let cases = vec![d, -d];

        for d in cases {
            let s = d.to_string();
            let actual: Result<Duration, _> = s.parse();
            assert!(actual.is_ok());
            assert_eq!(actual.unwrap(), d);
        }
    }

    #[apply(cases_for_display)]
    fn test_serialize(d: Duration) {
        let cases = vec![d, -d];

        for d in cases {
            let s = serde_json::to_string(&d).unwrap();
            let exp = d.to_string();
            assert_eq!(s, format!("\"{}\"", exp));
        }
    }

    #[apply(cases_for_parse)]
    fn test_deserialize(s: &str, expected: Option<Duration>) {
        let cases = vec![
            (format!("\"{}\"", s), expected),
            (format!("\"+{}\"", s), expected),
            (format!("\"-{}\"", s), expected.map(|d| -d)),
        ];

        for (s, expected) in cases {
            let actual: Result<Duration, _> = serde_json::from_str(&s);
            match expected {
                Some(expected) => {
                    assert!(actual.is_ok());
                    assert_eq!(actual.unwrap(), expected);
                }
                None => {
                    assert!(actual.is_err());
                }
            }
        }
    }

    #[test]
    fn test_neg() {
        let tested = -Duration::with_days(1);
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let tested = -Duration::with_days(-1);
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        let tested = -Duration::zero();
        let expected = Duration::zero();
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_add() {
        let tested = Duration::with_days(1) + Duration::with_days(2);
        let expected = Duration::with_days(3);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(1) + Duration::with_days(-2);
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(-1) + Duration::with_days(2);
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(-1) + Duration::with_days(-2);
        let expected = Duration::with_days(-3);
        assert_eq!(tested, expected);

        // days + seconds
        let tested = Duration::with_days(1) + Duration::with_secs(1);
        let expected = Duration::with_secs(24 * 60 * 60 + 1);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_sub() {
        let tested = Duration::with_days(1) - Duration::with_days(2);
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(1) - Duration::with_days(-2);
        let expected = Duration::with_days(3);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(-1) - Duration::with_days(2);
        let expected = Duration::with_days(-3);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(-1) - Duration::with_days(-2);
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        // days - seconds
        let tested = Duration::with_days(1) - Duration::with_secs(1);
        let expected = Duration::with_secs(24 * 60 * 60 - 1);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_add_assign() {
        let mut tested = Duration::with_days(1);
        tested += Duration::with_days(2);
        let expected = Duration::with_days(3);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(1);
        tested += Duration::with_days(-2);
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(-1);
        tested += Duration::with_days(2);
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(-1);
        tested += Duration::with_days(-2);
        let expected = Duration::with_days(-3);
        assert_eq!(tested, expected);

        // days and second
        let mut tested = Duration::with_days(1);
        tested += Duration::with_secs(1);
        let expected = Duration::with_secs(24 * 60 * 60 + 1);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_sub_assign() {
        let mut tested = Duration::with_days(1);
        tested -= Duration::with_days(2);
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(1);
        tested -= Duration::with_days(-2);
        let expected = Duration::with_days(3);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(-1);
        tested -= Duration::with_days(2);
        let expected = Duration::with_days(-3);
        assert_eq!(tested, expected);

        let mut tested = Duration::with_days(-1);
        tested -= Duration::with_days(-2);
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        // days and second
        let mut tested = Duration::with_days(1);
        tested -= Duration::with_secs(1);
        let expected = Duration::with_secs(24 * 60 * 60 - 1);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_mul() {
        let tested = Duration::with_days(1) * 2;
        let expected = Duration::with_days(2);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(1) * -2;
        let expected = Duration::with_days(-2);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_div() {
        let tested = Duration::with_days(2) / 2;
        let expected = Duration::with_days(1);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(2) / -2;
        let expected = Duration::with_days(-1);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(1) / 2;
        let expected = Duration::with_hours(12);
        assert_eq!(tested, expected);

        let tested = Duration::with_days(1) / -2;
        let expected = Duration::with_hours(-12);
        assert_eq!(tested, expected);

        let tested = Duration::with_hours(1) / 2;
        let expected = Duration::with_mins(30);
        assert_eq!(tested, expected);

        let tested = Duration::with_hours(1) / -2;
        let expected = Duration::with_mins(-30);
        assert_eq!(tested, expected);
    }

    #[test]
    fn test_velocity() {
        let tested = 1.0 / Duration::with_secs(1);
        let expected = Velocity::new(1.0, Duration::with_secs(1));
        assert_eq!(tested, expected);

        let tested = 1.0 / &Duration::with_secs(1);
        let expected = Velocity::new(1.0, Duration::with_secs(1));
        assert_eq!(tested, expected);

        let tested = 1.0 / Duration::with_mins(1);
        let expected = Velocity::new(1.0, Duration::with_mins(1));
        assert_eq!(tested, expected);

        let tested = 1.0 / &Duration::with_mins(1);
        let expected = Velocity::new(1.0, Duration::with_mins(1));
        assert_eq!(tested, expected);

        let tested = 1.0 / Duration::with_days(1);
        let expected = Velocity::new(1.0, Duration::with_days(1));
        assert_eq!(tested, expected);
    }
}
