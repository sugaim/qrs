use std::{
    fmt::{Debug, Display},
    ops::Sub,
    str::FromStr,
};

use chrono::{format::DelayedFormat, TimeZone};
use serde::{Deserialize, Serialize};

/// Thin wrapper around [chrono::DateTime] to override some traits
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime<Tz: chrono::TimeZone> {
    internal: chrono::DateTime<Tz>,
}

impl<Tz: TimeZone> Copy for DateTime<Tz> where Tz::Offset: Copy {}

// -----------------------------------------------------------------------------
// Display, Serde
// -----------------------------------------------------------------------------
impl<Tz: TimeZone> Debug for DateTime<Tz> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.internal, f)
    }
}

impl<Tz: TimeZone> Display for DateTime<Tz>
where
    Tz::Offset: Display,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.internal.format("%Y-%m-%dT%H:%M:%S%:z"))
    }
}

impl<Tz: TimeZone> Serialize for DateTime<Tz>
where
    Tz::Offset: Serialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.internal.serialize(serializer)
    }
}

impl<'de, Tz: TimeZone> Deserialize<'de> for DateTime<Tz>
where
    chrono::DateTime<Tz>: Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        chrono::DateTime::deserialize(deserializer).map(Into::into)
    }
}

impl<Tz: TimeZone> DateTime<Tz> {
    #[inline]
    pub fn to_rfc3339(&self) -> String {
        self.internal.to_rfc3339()
    }
}

impl<Tz: TimeZone> DateTime<Tz>
where
    Tz::Offset: Display,
{
    #[inline]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<chrono::format::StrftimeItems<'a>> {
        self.internal.format(fmt)
    }
}

// -----------------------------------------------------------------------------
// Constructors, Converters
// -----------------------------------------------------------------------------
impl<Tz: TimeZone> From<chrono::DateTime<Tz>> for DateTime<Tz> {
    #[inline]
    fn from(internal: chrono::DateTime<Tz>) -> Self {
        Self { internal }
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for chrono::DateTime<Tz> {
    #[inline]
    fn from(dt: DateTime<Tz>) -> Self {
        dt.internal
    }
}

impl<Tz: TimeZone> DateTime<Tz> {
    #[inline]
    pub fn new(datetime: chrono::NaiveDateTime, tz: Tz) -> Self {
        tz.from_local_datetime(&datetime).single().unwrap().into()
    }
}

impl<Tz: TimeZone> FromStr for DateTime<Tz>
where
    chrono::DateTime<Tz>: FromStr,
{
    type Err = <chrono::DateTime<Tz> as FromStr>::Err;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        chrono::DateTime::from_str(s).map(|dt| dt.into())
    }
}

// -----------------------------------------------------------------------------
// Accessors
// -----------------------------------------------------------------------------
impl<Tz: TimeZone> chrono::Datelike for DateTime<Tz> {
    #[inline]
    fn year(&self) -> i32 {
        self.internal.year()
    }
    #[inline]
    fn month(&self) -> u32 {
        self.internal.month()
    }
    #[inline]
    fn month0(&self) -> u32 {
        self.internal.month0()
    }
    #[inline]
    fn day(&self) -> u32 {
        self.internal.day()
    }
    #[inline]
    fn ordinal(&self) -> u32 {
        self.internal.ordinal()
    }
    #[inline]
    fn weekday(&self) -> chrono::Weekday {
        self.internal.weekday()
    }
    #[inline]
    fn iso_week(&self) -> chrono::IsoWeek {
        self.internal.iso_week()
    }
    #[inline]
    fn day0(&self) -> u32 {
        self.internal.day0()
    }
    #[inline]
    fn ordinal0(&self) -> u32 {
        self.internal.ordinal0()
    }
    #[inline]
    fn with_day(&self, day: u32) -> Option<Self> {
        self.internal.with_day(day).map(|dt| dt.into())
    }
    #[inline]
    fn with_day0(&self, day0: u32) -> Option<Self> {
        self.internal.with_day0(day0).map(|dt| dt.into())
    }
    #[inline]
    fn with_month(&self, month: u32) -> Option<Self> {
        self.internal.with_month(month).map(|dt| dt.into())
    }
    #[inline]
    fn with_month0(&self, month0: u32) -> Option<Self> {
        self.internal.with_month0(month0).map(|dt| dt.into())
    }
    #[inline]
    fn with_year(&self, year: i32) -> Option<Self> {
        self.internal.with_year(year).map(|dt| dt.into())
    }
    #[inline]
    fn with_ordinal(&self, ordinal: u32) -> Option<Self> {
        self.internal.with_ordinal(ordinal).map(|dt| dt.into())
    }
    #[inline]
    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self> {
        self.internal.with_ordinal0(ordinal0).map(|dt| dt.into())
    }
}
impl<Tz: TimeZone> chrono::Timelike for DateTime<Tz> {
    #[inline]
    fn hour(&self) -> u32 {
        self.internal.hour()
    }
    #[inline]
    fn minute(&self) -> u32 {
        self.internal.minute()
    }
    #[inline]
    fn second(&self) -> u32 {
        self.internal.second()
    }
    #[inline]
    fn nanosecond(&self) -> u32 {
        self.internal.nanosecond()
    }
    #[inline]
    fn with_hour(&self, hour: u32) -> Option<Self> {
        self.internal.with_hour(hour).map(|dt| dt.into())
    }
    #[inline]
    fn with_minute(&self, min: u32) -> Option<Self> {
        self.internal.with_minute(min).map(|dt| dt.into())
    }
    #[inline]
    fn with_second(&self, sec: u32) -> Option<Self> {
        self.internal.with_second(sec).map(|dt| dt.into())
    }
    #[inline]
    fn with_nanosecond(&self, nano: u32) -> Option<Self> {
        self.internal.with_nanosecond(nano).map(|dt| dt.into())
    }
}

impl<Tz: TimeZone> DateTime<Tz> {
    /// Returns a reference to the underlying [chrono::DateTime] object.
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// let chrono_obj = chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    ///
    /// assert_eq!(dt.as_chrono(), &chrono_obj);
    /// ```
    #[inline]
    pub fn as_chrono(&self) -> &chrono::DateTime<Tz> {
        &self.internal
    }

    /// Same as [chrono::DateTime::naive_local]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.local(), chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap().and_hms_opt(10, 42, 11).unwrap());
    /// ```
    #[inline]
    pub fn local(&self) -> chrono::NaiveDateTime {
        self.internal.naive_local()
    }

    /// Same as [chrono::DateTime::date_naive]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.date(), chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    /// ```
    #[inline]
    pub fn date(&self) -> chrono::NaiveDate {
        self.internal.date_naive()
    }

    /// Same as [chrono::DateTime::time]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.time(), chrono::NaiveTime::from_hms_opt(10, 42, 11).unwrap());
    /// ```
    #[inline]
    pub fn time(&self) -> chrono::NaiveTime {
        self.internal.time()
    }

    /// Same as [chrono::DateTime::offset]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.offset(), &chrono::FixedOffset::east(9 * 3600));
    /// ```
    #[inline]
    pub fn offset(&self) -> &Tz::Offset {
        self.internal.offset()
    }

    /// Same as [chrono::DateTime::with_timezone]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// let dt_utc = dt.with_timezone(&chrono::Utc);
    ///
    /// assert_eq!(dt_utc.to_string(), "2021-01-01T01:42:11+00:00");
    /// ```
    #[inline]
    pub fn with_timezone<U: TimeZone>(&self, tz: &U) -> DateTime<U> {
        self.internal.with_timezone(tz).into()
    }

    /// Same as [chrono::DateTime::timestamp]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.timestamp(), dt.as_chrono().timestamp());
    /// ```
    #[inline]
    pub fn timestamp(&self) -> i64 {
        self.internal.timestamp()
    }

    /// Same as [chrono::DateTime::timestamp_millis]
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(dt.timestamp_millis(), dt.as_chrono().timestamp_millis());
    /// ```
    #[inline]
    pub fn timestamp_millis(&self) -> i64 {
        self.internal.timestamp_millis()
    }
}

// -----------------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------------
impl<Tz: TimeZone> Sub for DateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        (self.internal - rhs.internal).into()
    }
}
impl<Tz: TimeZone + Clone> Sub<&DateTime<Tz>> for DateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: &Self) -> Self::Output {
        (self.internal - rhs.internal.clone()).into()
    }
}

macro_rules! define_self_duration_op {
    ($op:ident, $op_fn:ident) => {
        impl<Tz: TimeZone> std::ops::$op<super::Duration> for DateTime<Tz> {
            type Output = DateTime<Tz>;

            #[inline]
            fn $op_fn(self, rhs: super::Duration) -> Self::Output {
                (self.internal.$op_fn(*rhs.as_chrono())).into()
            }
        }
        impl<Tz: TimeZone> std::ops::$op<&super::Duration> for DateTime<Tz> {
            type Output = DateTime<Tz>;

            #[inline]
            fn $op_fn(self, rhs: &super::Duration) -> Self::Output {
                (self.internal.$op_fn(*rhs.as_chrono())).into()
            }
        }
    };
}
define_self_duration_op!(Add, add);
define_self_duration_op!(Sub, sub);
