use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::Sub,
    str::FromStr,
};

use chrono::{format::DelayedFormat, TimeZone};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// DateTime
//

/// Thin wrapper around [chrono::DateTime] to override some traits
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime<Tz: chrono::TimeZone> {
    internal: chrono::DateTime<Tz>,
}

impl<Tz: TimeZone> Copy for DateTime<Tz> where Tz::Offset: Copy {}

//
// display, serde
//
impl<Tz: TimeZone> Debug for DateTime<Tz> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.internal, f)
    }
}

impl<Tz: TimeZone> Display for DateTime<Tz>
where
    chrono::DateTime<Tz>: Display,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.internal, f)
    }
}

impl<Tz: TimeZone> Serialize for DateTime<Tz> {
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

impl<Tz: TimeZone> JsonSchema for DateTime<Tz> {
    fn schema_name() -> String {
        "DateTime".to_string()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("qcore::chrono::DateTime")
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = <chrono::DateTime<Tz> as JsonSchema>::json_schema(gen).into_object();
        res.metadata().description = Some("A datetime with timezone".to_string());
        res.metadata().title = Some(Self::schema_name());
        res.metadata().id = Some(Self::schema_id().into_owned());
        res.into()
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
    /// Formats the combined date and time with the specified format string.
    /// See [chrono::DateTime::format] for more details.
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// assert_eq!(format!("{}", dt.format("%Y-%m-%d %H:%M:%S %:z")), "2021-01-01 10:42:11 +09:00");
    /// assert_eq!(format!("{}", dt.format("%Y%m%dT%H%M%S%z")), "20210101T104211+0900")
    /// ```
    #[inline]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<chrono::format::StrftimeItems<'a>> {
        self.internal.format(fmt)
    }
}

//
// construction
//
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

//
// getters
//
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
    /// assert_eq!(dt_utc, chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T01:42:11Z").unwrap().into());
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

//
// operators
//
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
impl<Tz: TimeZone + Clone> Sub<DateTime<Tz>> for &DateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: DateTime<Tz>) -> Self::Output {
        (self.internal.clone() - rhs.internal).into()
    }
}
impl<Tz: TimeZone + Clone> Sub<&DateTime<Tz>> for &DateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: &DateTime<Tz>) -> Self::Output {
        (self.internal.clone() - rhs.internal.clone()).into()
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

// =============================================================================
#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};
    use num::Zero;

    use super::super::Duration;
    use super::*;

    #[test]
    fn test_debug() {
        // fixed offset
        let dt: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T10:42:11+09:00");

        // utc
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T10:42:11Z");

        // IANA
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T05:42:11EST");
    }

    #[test]
    fn test_display() {
        // fixed offset
        let dt: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();

        assert_eq!(dt.to_string(), "2021-01-01 10:42:11 +09:00");

        // utc
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();

        assert_eq!(dt.to_string(), "2021-01-01 10:42:11 UTC");

        // IANA
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();

        assert_eq!(dt.to_string(), "2021-01-01 05:42:11 EST");
    }

    #[test]
    fn test_serialize() {
        // fixed offset
        let dt: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(serialized, r#""2021-01-01T10:42:11+09:00""#);

        // utc
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(serialized, r#""2021-01-01T10:42:11Z""#);
    }

    #[test]
    fn test_deserialize() {
        // fixed offset
        let serialized = r#""2021-01-01T10:42:11+09:00""#;
        let deserialized: DateTime<chrono::FixedOffset> = serde_json::from_str(serialized).unwrap();
        let expected: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(deserialized, expected);

        // utc
        let serialized = r#""2021-01-01T10:42:11Z""#;
        let deserialized: DateTime<chrono::Utc> = serde_json::from_str(serialized).unwrap();
        let expected: DateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_new() {
        // fixed offset
        let dt = DateTime::<chrono::FixedOffset>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        assert_eq!(chrono_dt, expected);

        // utc
        let dt = DateTime::<chrono::Utc>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::Utc,
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z").unwrap();
        assert_eq!(chrono_dt, expected);

        // IANA
        let dt = DateTime::<chrono::Utc>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::Utc,
        )
        .with_timezone(&chrono_tz::Tz::America__New_York);

        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        assert_eq!(chrono_dt, expected);
    }

    #[test]
    fn test_sub() {
        // fixed offset
        let dt1: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let dt2: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(&dt1 - &dt2, dt1 - dt2);
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);

        let dt1: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();

        let dt2: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:12+09:00")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(&dt1 - &dt2, dt1 - dt2);
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);

        // utc
        let dt1: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        let dt2: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        let dt1: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        let dt2: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
            .unwrap()
            .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        // IANA
        let dt1: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        let dt2: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        let dt1: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        let dt2: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        // between summer time and winter time
        let dt1: DateTime<_> = chrono::NaiveDateTime::from_str("2021-03-13T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        let dt2: DateTime<_> = chrono::NaiveDateTime::from_str("2021-03-14T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        assert_eq!(dt2 - dt1, Duration::with_secs(23 * 60 * 60));
        assert_eq!(&dt2 - &dt1, dt2 - dt1);
        assert_eq!(dt2 - &dt1, dt2 - dt1);
        assert_eq!(&dt2 - dt1, dt2 - dt1);
    }

    #[test]
    fn test_add_sub_duration() {
        // fixed offset
        let dt: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);
        assert_eq!(dt - duration, dt);
        assert_eq!(dt - &duration, dt);

        let dt: DateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let duration = Duration::with_secs(1);
        assert_eq!(
            dt + duration,
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:12+09:00")
                .unwrap()
                .into()
        );
        assert_eq!(dt + &duration, dt + duration);
        assert_eq!(
            dt - duration,
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:10+09:00")
                .unwrap()
                .into()
        );
        assert_eq!(dt - &duration, dt - duration);

        // utc
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);
        assert_eq!(dt - duration, dt);
        assert_eq!(dt - &duration, dt);

        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .into();
        let duration = Duration::with_secs(1);
        assert_eq!(
            dt + duration,
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
                .unwrap()
                .into()
        );
        assert_eq!(dt + &duration, dt + duration);
        assert_eq!(
            dt - duration,
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:10Z")
                .unwrap()
                .into()
        );
        assert_eq!(dt - &duration, dt - duration);

        // IANA
        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);

        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        let duration = Duration::with_secs(1);
        assert_eq!(
            dt + duration,
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into()
        );
        assert_eq!(dt + &duration, dt + duration);

        let dt: DateTime<_> = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York)
            .into();
        let duration = Duration::with_secs(1);
        assert_eq!(
            dt - duration,
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:10Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into()
        );
        assert_eq!(dt - &duration, dt - duration);

        // between summer time and winter time
        let dt: DateTime<_> = chrono::NaiveDateTime::from_str("2021-03-13T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        let duration = Duration::with_secs(23 * 60 * 60);
        assert_eq!(
            dt + duration,
            chrono::NaiveDateTime::from_str("2021-03-14T08:30:00")
                .unwrap()
                .and_local_timezone(chrono_tz::Tz::America__New_York)
                .single()
                .unwrap()
                .into()
        );
        assert_eq!(dt + &duration, dt + duration);

        let dt: DateTime<_> = chrono::NaiveDateTime::from_str("2021-03-14T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        let duration = Duration::with_secs(23 * 60 * 60);
        assert_eq!(
            dt - duration,
            chrono::NaiveDateTime::from_str("2021-03-13T08:30:00")
                .unwrap()
                .and_local_timezone(chrono_tz::Tz::America__New_York)
                .single()
                .unwrap()
                .into()
        );
        assert_eq!(dt - &duration, dt - duration);
    }

    #[test]
    fn test_datelike() {
        // fixed offset
        let chrono_dts = vec![
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap(),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-04-01T10:42:11+09:00").unwrap(),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-12-31T10:42:11+09:00").unwrap(),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.year(), chrono_dt.year());
            assert_eq!(dt.month(), chrono_dt.month());
            assert_eq!(dt.month0(), chrono_dt.month0());
            assert_eq!(dt.day(), chrono_dt.day());
            assert_eq!(dt.ordinal(), chrono_dt.ordinal());
            assert_eq!(dt.weekday(), chrono_dt.weekday());
            assert_eq!(dt.iso_week(), chrono_dt.iso_week());
            assert_eq!(dt.day0(), chrono_dt.day0());
            assert_eq!(dt.ordinal0(), chrono_dt.ordinal0());
            assert_eq!(dt.with_day(1), chrono_dt.with_day(1).map(|dt| dt.into()));
            assert_eq!(dt.with_day0(0), chrono_dt.with_day0(0).map(|dt| dt.into()));
            assert_eq!(
                dt.with_month(4),
                chrono_dt.with_month(4).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_month0(3),
                chrono_dt.with_month0(3).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_year(2022),
                chrono_dt.with_year(2022).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal(365),
                chrono_dt.with_ordinal(365).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal0(364),
                chrono_dt.with_ordinal0(364).map(|dt| dt.into())
            );
        }

        // utc
        let chrono_dts = vec![
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z").unwrap(),
            chrono::DateTime::<chrono::Utc>::from_str("2021-04-01T10:42:11Z").unwrap(),
            chrono::DateTime::<chrono::Utc>::from_str("2021-12-31T10:42:11Z").unwrap(),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.year(), chrono_dt.year());
            assert_eq!(dt.month(), chrono_dt.month());
            assert_eq!(dt.month0(), chrono_dt.month0());
            assert_eq!(dt.day(), chrono_dt.day());
            assert_eq!(dt.ordinal(), chrono_dt.ordinal());
            assert_eq!(dt.weekday(), chrono_dt.weekday());
            assert_eq!(dt.iso_week(), chrono_dt.iso_week());
            assert_eq!(dt.day0(), chrono_dt.day0());
            assert_eq!(dt.ordinal0(), chrono_dt.ordinal0());
            assert_eq!(dt.with_day(1), chrono_dt.with_day(1).map(|dt| dt.into()));
            assert_eq!(dt.with_day0(0), chrono_dt.with_day0(0).map(|dt| dt.into()));
            assert_eq!(
                dt.with_month(4),
                chrono_dt.with_month(4).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_month0(3),
                chrono_dt.with_month0(3).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_year(2022),
                chrono_dt.with_year(2022).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal(365),
                chrono_dt.with_ordinal(365).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal0(364),
                chrono_dt.with_ordinal0(364).map(|dt| dt.into())
            );
        }

        // IANA
        let chrono_dts = vec![
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
            chrono::DateTime::<chrono::Utc>::from_str("2021-04-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
            chrono::DateTime::<chrono::Utc>::from_str("2021-12-31T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.year(), chrono_dt.year());
            assert_eq!(dt.month(), chrono_dt.month());
            assert_eq!(dt.month0(), chrono_dt.month0());
            assert_eq!(dt.day(), chrono_dt.day());
            assert_eq!(dt.ordinal(), chrono_dt.ordinal());
            assert_eq!(dt.weekday(), chrono_dt.weekday());
            assert_eq!(dt.iso_week(), chrono_dt.iso_week());
            assert_eq!(dt.day0(), chrono_dt.day0());
            assert_eq!(dt.ordinal0(), chrono_dt.ordinal0());
            assert_eq!(dt.with_day(1), chrono_dt.with_day(1).map(|dt| dt.into()));
            assert_eq!(dt.with_day0(0), chrono_dt.with_day0(0).map(|dt| dt.into()));
            assert_eq!(
                dt.with_month(4),
                chrono_dt.with_month(4).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_month0(3),
                chrono_dt.with_month0(3).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_year(2022),
                chrono_dt.with_year(2022).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal(365),
                chrono_dt.with_ordinal(365).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_ordinal0(364),
                chrono_dt.with_ordinal0(364).map(|dt| dt.into())
            );
        }
    }

    #[test]
    fn test_timelike() {
        // fixed offset
        let chrono_dts = vec![
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap(),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-04-01T10:42:11+09:00").unwrap(),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-12-31T10:42:11+09:00").unwrap(),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.hour(), chrono_dt.hour());
            assert_eq!(dt.minute(), chrono_dt.minute());
            assert_eq!(dt.second(), chrono_dt.second());
            assert_eq!(dt.nanosecond(), chrono_dt.nanosecond());
            assert_eq!(dt.with_hour(1), chrono_dt.with_hour(1).map(|dt| dt.into()));
            assert_eq!(
                dt.with_minute(1),
                chrono_dt.with_minute(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_second(1),
                chrono_dt.with_second(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_nanosecond(1),
                chrono_dt.with_nanosecond(1).map(|dt| dt.into())
            );
        }

        // utc
        let chrono_dts = vec![
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z").unwrap(),
            chrono::DateTime::<chrono::Utc>::from_str("2021-04-01T10:42:11Z").unwrap(),
            chrono::DateTime::<chrono::Utc>::from_str("2021-12-31T10:42:11Z").unwrap(),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.hour(), chrono_dt.hour());
            assert_eq!(dt.minute(), chrono_dt.minute());
            assert_eq!(dt.second(), chrono_dt.second());
            assert_eq!(dt.nanosecond(), chrono_dt.nanosecond());
            assert_eq!(dt.with_hour(1), chrono_dt.with_hour(1).map(|dt| dt.into()));
            assert_eq!(
                dt.with_minute(1),
                chrono_dt.with_minute(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_second(1),
                chrono_dt.with_second(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_nanosecond(1),
                chrono_dt.with_nanosecond(1).map(|dt| dt.into())
            );
        }

        // IANA
        let chrono_dts = vec![
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
            chrono::DateTime::<chrono::Utc>::from_str("2021-04-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
            chrono::DateTime::<chrono::Utc>::from_str("2021-12-31T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime<_> = chrono_dt.into();
            assert_eq!(dt.hour(), chrono_dt.hour());
            assert_eq!(dt.minute(), chrono_dt.minute());
            assert_eq!(dt.second(), chrono_dt.second());
            assert_eq!(dt.nanosecond(), chrono_dt.nanosecond());
            assert_eq!(dt.with_hour(1), chrono_dt.with_hour(1).map(|dt| dt.into()));
            assert_eq!(
                dt.with_minute(1),
                chrono_dt.with_minute(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_second(1),
                chrono_dt.with_second(1).map(|dt| dt.into())
            );
            assert_eq!(
                dt.with_nanosecond(1),
                chrono_dt.with_nanosecond(1).map(|dt| dt.into())
            );
        }
    }
}
