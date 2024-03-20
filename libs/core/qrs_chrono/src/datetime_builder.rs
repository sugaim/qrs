use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;
use chrono::{LocalResult, NaiveDate, NaiveTime, TimeZone};

use crate::{DateTime, TimeCut, Tz};

// -----------------------------------------------------------------------------
// DateTimeBuildError
//
/// Error reported by [DateTimeBuilder::build]
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Hash)]
pub enum DateTimeBuildError {
    /// Due to invalid date
    #[error("Invalid date: ymd=({}, {}, {})", .year, .month, .day)]
    Date { year: i32, month: u32, day: u32 },

    /// Due to invalid time
    #[error("Invalid time: hms.f=({}, {}, {}, {})", .hour, .minute, .second, .nanosecond)]
    Time {
        hour: u32,
        minute: u32,
        second: u32,
        nanosecond: u32,
    },

    /// Due to invalid fixed offset
    #[error("Invalid fixed offset: offset_sec={}", .offset_sec)]
    FixedOffset { offset_sec: i32 },

    /// Due to invalid timezone string
    #[error("Parse error: {}", .0)]
    Tz(String),

    /// Specified timepoint does not exist, e.g., due to daylight saving time transition
    #[error("Specified timepoint does not exist, e.g., due to daylight saving time transition")]
    NotExist,

    /// Specified timepoint is ambiguous, e.g., due to daylight saving time transition
    #[error("Specified timepoint is ambiguous, e.g., due to daylight saving time transition")]
    Ambiguous,
}

// -----------------------------------------------------------------------------
// DateTimeBuilder
//

/// A builder for creating a `qrs_chrono::DateTime`.
///
/// # Example
/// ```
/// use qrs_chrono::DateTimeBuilder;
///
/// let datetime = DateTimeBuilder::new()
///     .with_ymd(2021, 1, 1)
///     .with_hms(10, 42, 11)
///     .with_fixed_offset(9 * 3600)
///     .build()
///     .unwrap();
///
/// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTimeBuilder<D = (), T = (), Tz = ()>(Result<(D, T, Tz), DateTimeBuildError>);

/// Time cut, such as Tokyo close.
///
/// This is just a type alias of [DateTimeBuilder] with time and timezone.
/// So only date part is missing, and this can convert date into [DateTime].
pub type DateToDateTime<Tz = super::Tz> = DateTimeBuilder<(), NaiveTime, Tz>;

//
// display, serde
//
impl<Tz> Display for DateTimeBuilder<(), NaiveTime, Tz>
where
    Tz: Display + chrono::TimeZone,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner() {
            Ok((_, time, tz)) => write!(f, "{}{}", time.format("%H:%M:%S%.f"), tz),
            Err(err) => write!(f, "Invalid DateTimeBuilder: {}", err),
        }
    }
}

#[cfg(feature = "serde")]
impl<Tz> serde::Serialize for DateTimeBuilder<(), NaiveTime, Tz>
where
    Tz: Display + chrono::TimeZone,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.is_err() {
            return Err(serde::ser::Error::custom(format!(
                "Invalid DateTimeBuilder: {}",
                self.0.as_ref().err().unwrap()
            )));
        }
        let s = format!("{}", self);
        serializer.serialize_str(&s)
    }
}

#[cfg(feature = "serde")]
impl<'de, Tz> serde::Deserialize<'de> for DateTimeBuilder<(), NaiveTime, Tz>
where
    Tz: chrono::TimeZone,
    DateTime<Tz>: FromStr,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTimeBuilder::from_str(&s).map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl Default for DateTimeBuilder {
    #[inline]
    fn default() -> Self {
        Self(Ok(((), (), ())))
    }
}

impl DateTimeBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Tz> FromStr for DateTimeBuilder<(), NaiveTime, Tz>
where
    Tz: chrono::TimeZone,
    DateTime<Tz>: FromStr,
{
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let maybe_datetime = format!("2021-01-01T{}", s);
        let Ok(dt): Result<DateTime<Tz>, _> = maybe_datetime.parse() else {
            return Err(anyhow!(
                "Fail to parse time and timezone from string: {s}. The string must be valid when it is concatenated with date string and time separator, 'yyyy-MM-ddT'",
            ));
        };
        Ok(DateTimeBuilder::new()
            .with_time(&dt.time())
            .with_timezone(dt.timezone()))
    }
}

//
// methods
//
impl<D, T, Tz> DateTimeBuilder<D, T, Tz> {
    #[inline]
    pub fn into_inner(self) -> Result<(D, T, Tz), DateTimeBuildError> {
        self.0
    }

    #[inline]
    pub fn inner(&self) -> &Result<(D, T, Tz), DateTimeBuildError> {
        &self.0
    }
}

impl<D, Tz> DateTimeBuilder<D, (), Tz> {
    /// Set time to the builder.
    /// Available types are implementations of [chrono::Timelike].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_time(&chrono::NaiveTime::from_hms_opt(10, 42, 11).unwrap())
    ///     .with_fixed_offset(9 * 3600)
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_time<T>(self, time: &T) -> DateTimeBuilder<D, NaiveTime, Tz>
    where
        T: chrono::Timelike,
    {
        DateTimeBuilder(self.0.and_then(|(date, _, tz)| {
            NaiveTime::from_hms_nano_opt(
                time.hour(),
                time.minute(),
                time.second(),
                time.nanosecond(),
            )
            .ok_or_else(|| DateTimeBuildError::Time {
                hour: time.hour(),
                minute: time.minute(),
                second: time.second(),
                nanosecond: time.nanosecond(),
            })
            .map(|time| (date, time, tz))
        }))
    }

    /// Set time to the builder.
    /// Invalid time is captured when build is called.
    /// Details for invalid time is described in [chrono::NaiveTime::from_hms_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_fixed_offset(9 * 3600)
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_hms(
        self,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> DateTimeBuilder<D, NaiveTime, Tz> {
        DateTimeBuilder(self.0.and_then(|(date, _, tz)| {
            NaiveTime::from_hms_opt(hour, minute, second)
                .ok_or(DateTimeBuildError::Time {
                    hour,
                    minute,
                    second,
                    nanosecond: 0,
                })
                .map(|time| (date, time, tz))
        }))
    }
}
impl<T, Tz> DateTimeBuilder<(), T, Tz> {
    /// Set date to the builder.
    /// Available types are implementations of [chrono::Datelike].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_date(&chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap())
    ///     .with_hms(10, 42, 11)
    ///     .with_utc()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+00:00");
    /// ```
    #[inline]
    pub fn with_date<D>(self, date: &D) -> DateTimeBuilder<NaiveDate, T, Tz>
    where
        D: chrono::Datelike,
    {
        DateTimeBuilder(self.0.and_then(|(_, time, tz)| {
            NaiveDate::from_num_days_from_ce_opt(date.num_days_from_ce())
                .ok_or(DateTimeBuildError::Date {
                    year: date.year(),
                    month: date.month(),
                    day: date.day(),
                })
                .map(|date| (date, time, tz))
        }))
    }

    /// Set date to the builder.
    /// Invalid date is captured when build is called.
    /// Details for invalid date is described in [chrono::NaiveDate::from_ymd_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_fixed_offset(9 * 3600)
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_ymd(self, year: i32, month: u32, day: u32) -> DateTimeBuilder<NaiveDate, T, Tz> {
        DateTimeBuilder(self.0.and_then(|(_, time, tz)| {
            NaiveDate::from_ymd_opt(year, month, day)
                .ok_or(DateTimeBuildError::Date { year, month, day })
                .map(|date| (date, time, tz))
        }))
    }
}
impl<D, T> DateTimeBuilder<D, T, ()> {
    /// Set timezone to the builder.
    /// Available types are implementations of [chrono::TimeZone].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_timezone(chrono::FixedOffset::east_opt(9 * 3600).unwrap())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_timezone<Tz: TimeZone>(self, timezone: Tz) -> DateTimeBuilder<D, T, Tz> {
        DateTimeBuilder(self.0.map(|(date, time, _)| (date, time, timezone)))
    }

    /// Set fixed offset timezone to the builder.
    /// We use east offset rather than west one as argument because the offset is positive.
    ///
    /// Invalid offset is captured when build is called.
    /// Details for invalid offset is described in [chrono::FixedOffset::east_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_fixed_offset(9 * 3600)
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_fixed_offset(self, secs: i32) -> DateTimeBuilder<D, T, chrono::FixedOffset> {
        DateTimeBuilder(self.0.and_then(|(date, time, _)| {
            chrono::FixedOffset::east_opt(secs)
                .ok_or(DateTimeBuildError::FixedOffset { offset_sec: secs })
                .map(|tz| (date, time, tz))
        }))
    }

    /// Set UTC timezone to the builder.
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_utc()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+00:00");
    /// ```
    #[inline]
    pub fn with_utc(self) -> DateTimeBuilder<D, T, chrono::Utc> {
        self.with_timezone(chrono::Utc)
    }

    /// Set timezone from string to [`crate::Tz`]
    ///
    /// Available timezone strings are IANA timezone names and fixed offset strings.
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///    .with_ymd(2021, 1, 1)
    ///    .with_hms(10, 42, 11)
    ///    .with_parsed_timezone("Asia/Tokyo")
    ///    .build()
    ///    .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_parsed_timezone(self, tz: &str) -> DateTimeBuilder<D, T, Tz> {
        DateTimeBuilder(self.0.and_then(|(date, time, _)| {
            Tz::from_str(tz)
                .map_err(|e| DateTimeBuildError::Tz(e.to_string()))
                .map(|tz| (date, time, tz))
        }))
    }
}

//
// build
//
impl<Tz: TimeZone> DateTimeBuilder<NaiveDate, NaiveTime, Tz> {
    /// Build a `DateTime` from the builder with stored date, time and timezone.
    /// This methos is available only after setting date, time and timezone.
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1)
    ///     .with_hms(10, 42, 11)
    ///     .with_timezone(chrono::FixedOffset::east_opt(9 * 3600).unwrap())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn build(self) -> Result<DateTime<Tz>, DateTimeBuildError> {
        let (d, t, tz) = self.0?;
        match tz.from_local_datetime(&d.and_time(t)) {
            LocalResult::Single(dt) => Ok(dt.into()),
            LocalResult::None => Err(DateTimeBuildError::NotExist),
            LocalResult::Ambiguous(_, _) => Err(DateTimeBuildError::Ambiguous),
        }
    }
}

impl<Tz: chrono::TimeZone> TimeCut for DateTimeBuilder<(), NaiveTime, Tz> {
    type Err = DateTimeBuildError;
    type Tz = Tz;

    #[inline]
    fn to_datetime(&self, date: NaiveDate) -> Result<DateTime<Tz>, Self::Err> {
        match &self.0 {
            Ok((_, time, tz)) => match tz.from_local_datetime(&date.and_time(*time)) {
                LocalResult::Single(dt) => Ok(dt.into()),
                LocalResult::None => Err(DateTimeBuildError::NotExist),
                LocalResult::Ambiguous(_, _) => Err(DateTimeBuildError::Ambiguous),
            },
            Err(e) => Err(e.clone()),
        }
    }
}
