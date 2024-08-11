use std::{fmt::Display, str::FromStr, sync::OnceLock};

use anyhow::Context;
use chrono::{offset::LocalResult, NaiveDate};
use derivative::Derivative;
use schemars::schema::SchemaObject;
use serde::{Deserialize, Serialize};

use crate::{
    duration::{Duration, Tenor},
    timepoint::Tz,
};

// -----------------------------------------------------------------------------
// DateTime
// -----------------------------------------------------------------------------
/// Thin wrapper around [`chrono::DateTime`] with [`Tz`] timezone.
///
/// Mainly this struct is implemented to override some operators.
#[derive(Derivative, Clone)]
#[derivative(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    pub(crate) inner: chrono::DateTime<Tz>,
    #[cfg(debug_assertions)]
    #[derivative(Debug = "ignore", PartialEq = "ignore", PartialOrd = "ignore")]
    pub(crate) debug_str: String,
}

impl std::hash::Hash for DateTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

//
// conversion
//
impl From<DateTime> for chrono::DateTime<Tz> {
    #[inline]
    fn from(inner: DateTime) -> chrono::DateTime<Tz> {
        inner.inner
    }
}
impl From<chrono::DateTime<Tz>> for DateTime {
    #[inline]
    fn from(inner: chrono::DateTime<Tz>) -> Self {
        #[cfg(debug_assertions)]
        let debug_str = format!("{}", inner.to_rfc3339());

        #[cfg(debug_assertions)]
        return DateTime { inner, debug_str };

        #[cfg(not(debug_assertions))]
        return DateTime { inner };
    }
}
impl From<chrono::DateTime<chrono::Utc>> for DateTime {
    #[inline]
    fn from(inner: chrono::DateTime<chrono::Utc>) -> Self {
        inner.with_timezone(&Tz::Utc).into()
    }
}
impl From<chrono::DateTime<chrono::FixedOffset>> for DateTime {
    #[inline]
    fn from(inner: chrono::DateTime<chrono::FixedOffset>) -> Self {
        inner
            .with_timezone(&Tz::FixedOffset(*inner.offset()))
            .into()
    }
}
impl From<chrono::DateTime<chrono_tz::Tz>> for DateTime {
    #[inline]
    fn from(inner: chrono::DateTime<chrono_tz::Tz>) -> Self {
        inner.with_timezone(&Tz::Iana(inner.timezone())).into()
    }
}

//
// ser/de
//
impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt = chrono::SecondsFormat::AutoSi;
        match self.inner.timezone() {
            Tz::Utc => write!(f, "{}", self.inner.to_rfc3339_opts(fmt, true)),
            Tz::FixedOffset(_) => {
                write!(f, "{}", self.inner.to_rfc3339_opts(fmt, false))
            }
            Tz::Iana(tz) => write!(
                f,
                "{}[{}]",
                self.inner.to_rfc3339_opts(fmt, false),
                tz.name()
            ),
        }
    }
}

impl FromStr for DateTime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static WITH_TZ: OnceLock<regex::Regex> = OnceLock::new();
        let with_tz = WITH_TZ.get_or_init(|| {
            regex::Regex::new(r"^(?P<timepoint>[^\[\]]+)\[(?P<timezone>[^\[\]]+)\]$").unwrap()
        });

        if let Some(caps) = with_tz.captures(s) {
            let tp = &caps["timepoint"];
            let tz = &caps["timezone"];
            if let Some(tp) = chrono::DateTime::parse_from_rfc3339(tp).ok() {
                let tz = Tz::from_str(tz)
                    .with_context(|| format!("parse '{}' to timezone", &caps["timezone"]))?;
                return Ok(tp.with_timezone(&tz).into());
            } else if let Some(tp) =
                chrono::NaiveDateTime::parse_from_str(tp, "%Y-%m-%dT%H:%M:%S").ok()
            {
                let tz = Tz::from_str(tz)
                    .with_context(|| format!("parse '{}' to timezone", &caps["timezone"]))?;
                match tp.and_local_timezone(tz) {
                    chrono::LocalResult::Single(tp) => return Ok(tp.into()),
                    chrono::LocalResult::Ambiguous(_, _) => {
                        anyhow::bail!("parse '{}' to datetime. Ambiguous datetime", tp)
                    }
                    chrono::LocalResult::None => {
                        anyhow::bail!("parse '{}' to datetime. Invalid datetime", tp)
                    }
                }
            } else {
                anyhow::bail!("parse '{}' to datetime. Only RFC3339 string or naive datetime(%Y-%m-%dT%H:%M:%S) are supported", tp);
            }
        } else {
            let timeponint = chrono::DateTime::parse_from_rfc3339(s)
                .with_context(|| format!("parse '{}' to datetime", s))?;
            if s.ends_with('Z') {
                return Ok(timeponint.with_timezone(&Tz::Utc).into());
            }
            return Ok(timeponint.into());
        }
    }
}

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        DateTime::from_str(s).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for DateTime {
    fn schema_name() -> String {
        "DateTime".to_string()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qchrono::DateTime".into()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut sch = SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        sch.metadata().description = Some(
            "DateTime with timezone. RFC3339 string or naive datetime with IANA(e.g. '2024-06-01T12:34:56[Asia/Tokyo]') are supported"
                .to_string()
        );
        sch.string().pattern = Some(
            r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|(\+|-)\d{2}:\d{2}|\[.+\])$"
                .to_string(),
        );
        sch.into()
    }
}

//
// datetime behaviors
//
impl chrono::Datelike for DateTime {
    #[inline]
    fn year(&self) -> i32 {
        self.inner.year()
    }

    #[inline]
    fn month(&self) -> u32 {
        self.inner.month()
    }

    #[inline]
    fn month0(&self) -> u32 {
        self.inner.month0()
    }

    #[inline]
    fn day(&self) -> u32 {
        self.inner.day()
    }

    #[inline]
    fn ordinal(&self) -> u32 {
        self.inner.ordinal()
    }

    #[inline]
    fn weekday(&self) -> chrono::Weekday {
        self.inner.weekday()
    }

    #[inline]
    fn iso_week(&self) -> chrono::IsoWeek {
        self.inner.iso_week()
    }

    #[inline]
    fn with_day(&self, day: u32) -> Option<Self> {
        self.inner.with_day(day).map(|dt| dt.into())
    }

    #[inline]
    fn day0(&self) -> u32 {
        self.inner.day0()
    }

    #[inline]
    fn num_days_from_ce(&self) -> i32 {
        self.inner.num_days_from_ce()
    }

    #[inline]
    fn ordinal0(&self) -> u32 {
        self.inner.ordinal0()
    }

    #[inline]
    fn with_day0(&self, day0: u32) -> Option<Self> {
        self.inner.with_day0(day0).map(|dt| dt.into())
    }

    #[inline]
    fn with_month(&self, month: u32) -> Option<Self> {
        self.inner.with_month(month).map(|dt| dt.into())
    }

    #[inline]
    fn with_month0(&self, month0: u32) -> Option<Self> {
        self.inner.with_month0(month0).map(|dt| dt.into())
    }

    #[inline]
    fn with_year(&self, year: i32) -> Option<Self> {
        self.inner.with_year(year).map(|dt| dt.into())
    }

    #[inline]
    fn with_ordinal(&self, ordinal: u32) -> Option<Self> {
        self.inner.with_ordinal(ordinal).map(|dt| dt.into())
    }

    #[inline]
    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self> {
        self.inner.with_ordinal0(ordinal0).map(|dt| dt.into())
    }

    #[inline]
    fn year_ce(&self) -> (bool, u32) {
        self.inner.year_ce()
    }
}

impl chrono::Timelike for DateTime {
    #[inline]
    fn hour(&self) -> u32 {
        self.inner.hour()
    }

    #[inline]
    fn minute(&self) -> u32 {
        self.inner.minute()
    }

    #[inline]
    fn second(&self) -> u32 {
        self.inner.second()
    }

    #[inline]
    fn hour12(&self) -> (bool, u32) {
        self.inner.hour12()
    }

    #[inline]
    fn nanosecond(&self) -> u32 {
        self.inner.nanosecond()
    }

    #[inline]
    fn num_seconds_from_midnight(&self) -> u32 {
        self.inner.num_seconds_from_midnight()
    }

    #[inline]
    fn with_hour(&self, hour: u32) -> Option<Self> {
        self.inner.with_hour(hour).map(|dt| dt.into())
    }

    #[inline]
    fn with_minute(&self, min: u32) -> Option<Self> {
        self.inner.with_minute(min).map(|dt| dt.into())
    }

    #[inline]
    fn with_nanosecond(&self, nano: u32) -> Option<Self> {
        self.inner.with_nanosecond(nano).map(|dt| dt.into())
    }

    #[inline]
    fn with_second(&self, sec: u32) -> Option<Self> {
        self.inner.with_second(sec).map(|dt| dt.into())
    }
}

//
// operators
//
impl std::ops::Sub<DateTime> for DateTime {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: DateTime) -> Duration {
        (self.inner - rhs.inner).into()
    }
}

impl std::ops::Sub<&DateTime> for &DateTime {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: &DateTime) -> Duration {
        (self.inner - rhs.inner).into()
    }
}

impl std::ops::Sub<&DateTime> for DateTime {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: &DateTime) -> Duration {
        (self.inner - rhs.inner).into()
    }
}

impl std::ops::Add<Duration> for DateTime {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self {
        (self.inner + rhs.inner).into()
    }
}

impl std::ops::Sub<Duration> for DateTime {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Duration) -> Self {
        (self.inner - rhs.inner).into()
    }
}

//
// methods
//
impl DateTime {
    #[inline]
    pub fn date(&self) -> NaiveDate {
        self.inner.date_naive()
    }
    #[inline]
    pub fn time(&self) -> chrono::NaiveTime {
        self.inner.time()
    }

    #[inline]
    pub fn add_tenor(&self, tenor: Tenor) -> LocalResult<Self> {
        let dt = (self.date() + tenor).and_time(self.time());
        match dt.and_local_timezone(self.inner.timezone()) {
            chrono::LocalResult::Single(dt) => LocalResult::Single(dt.into()),
            chrono::LocalResult::Ambiguous(e, l) => LocalResult::Ambiguous(e.into(), l.into()),
            chrono::LocalResult::None => LocalResult::None,
        }
    }

    #[inline]
    pub fn timezone(&self) -> Tz {
        self.inner.timezone()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("1970-01-01T00:00:00Z")]
    #[case("2024-06-01T12:34:56Z")]
    #[case("2999-12-31T23:59:59Z")]
    fn test_parse_utc(#[case] s: &str) {
        let expected = chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&crate::timepoint::Tz::Utc)
            .into();
        let tested = DateTime::from_str(s).unwrap();

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case("1970-01-01T00:00:00+09:00")]
    #[case("2024-06-01T12:34:56+09:00")]
    #[case("2999-12-31T23:59:59+09:00")]
    #[case("1970-01-01T00:00:00-05:23")]
    #[case("2024-06-01T12:34:56-05:23")]
    #[case("2999-12-31T23:59:59-05:23")]
    fn test_parse_fixed_offset(#[case] s: &str) {
        let expected = chrono::DateTime::parse_from_rfc3339(s).unwrap().into();
        let tested = DateTime::from_str(s).unwrap();

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case("1970-01-01T00:00:00[Z]", "1970-01-01T00:00:00+00:00", "Z")]
    #[case("2024-06-01T12:34:56[Z]", "2024-06-01T12:34:56+00:00", "Z")]
    #[case("2999-12-31T23:59:59[Z]", "2999-12-31T23:59:59+00:00", "Z")]
    #[case("1970-01-01T00:00:00+09:00[Z]", "1970-01-01T00:00:00+09:00", "Z")]
    #[case("2024-06-01T12:34:56+09:00[Z]", "2024-06-01T12:34:56+09:00", "Z")]
    #[case("2999-12-31T23:59:59+09:00[Z]", "2999-12-31T23:59:59+09:00", "Z")]
    #[case("1970-01-01T00:00:00[+09:00]", "1970-01-01T00:00:00+09:00", "+09:00")]
    #[case("2024-06-01T12:34:56[+09:00]", "2024-06-01T12:34:56+09:00", "+09:00")]
    #[case("2999-12-31T23:59:59[+09:00]", "2999-12-31T23:59:59+09:00", "+09:00")]
    #[case(
        "1970-01-01T00:00:00+04:22[-05:23]",
        "1970-01-01T00:00:00+04:22",
        "-05:23"
    )]
    #[case(
        "2024-06-01T12:34:56+04:22[-05:23]",
        "2024-06-01T12:34:56+04:22",
        "-05:23"
    )]
    #[case(
        "2999-12-31T23:59:59+04:22[-05:23]",
        "2999-12-31T23:59:59+04:22",
        "-05:23"
    )]
    #[case("1970-01-01T00:00:00[-05:23]", "1970-01-01T00:00:00-05:23", "-05:23")]
    #[case("2024-06-01T12:34:56[-05:23]", "2024-06-01T12:34:56-05:23", "-05:23")]
    #[case("2999-12-31T23:59:59[-05:23]", "2999-12-31T23:59:59-05:23", "-05:23")]
    #[case("1970-01-01T00:00:00[UTC]", "1970-01-01T00:00:00Z", "UTC")]
    #[case("2024-06-01T12:34:56[UTC]", "2024-06-01T12:34:56Z", "UTC")]
    #[case("2999-12-31T23:59:59[UTC]", "2999-12-31T23:59:59Z", "UTC")]
    #[case(
        "1970-01-01T00:00:00-05:23[UTC]",
        "1970-01-01T00:00:00-05:23",
        "+09:00"
    )]
    #[case(
        "2024-06-01T12:34:56-05:23[UTC]",
        "2024-06-01T12:34:56-05:23",
        "+09:00"
    )]
    #[case(
        "2999-12-31T23:59:59-05:23[UTC]",
        "2999-12-31T23:59:59-05:23",
        "+09:00"
    )]
    #[case(
        "1970-01-01T00:00:00[Asia/Tokyo]",
        "1970-01-01T00:00:00+09:00",
        "Asia/Tokyo"
    )]
    #[case(
        "2024-06-01T12:34:56[Asia/Tokyo]",
        "2024-06-01T12:34:56+09:00",
        "Asia/Tokyo"
    )]
    #[case(
        "2999-12-31T23:59:59[Asia/Tokyo]",
        "2999-12-31T23:59:59+09:00",
        "Asia/Tokyo"
    )]
    #[case(
        "1970-01-01T00:00:00[America/New_York]",
        "1970-01-01T00:00:00-05:00",
        "America/New_York"
    )]
    #[case(
        "2024-06-01T12:34:56[America/New_York]",
        "2024-06-01T12:34:56-04:00",
        "America/New_York"
    )]
    #[case(
        "2024-12-01T12:34:56[America/New_York]",
        "2024-12-01T12:34:56-05:00",
        "America/New_York"
    )]
    #[case(
        "2999-12-31T23:59:59[America/New_York]",
        "2999-12-31T23:59:59-05:00",
        "America/New_York"
    )]
    fn test_parse_with_tz(#[case] s: &str, #[case] tp: &str, #[case] tz: &str) {
        let expected = chrono::DateTime::parse_from_rfc3339(tp)
            .unwrap()
            .with_timezone(&crate::timepoint::Tz::from_str(tz).unwrap())
            .into();
        let tested = DateTime::from_str(s).unwrap();

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case::no_tz("2024-06-01T12:34:56")]
    #[case::no_tz("2024-06-01T12:34:56.000000000")]
    #[case::invalid_tp("2024-06-01T12:34:56+09")]
    #[case::invalid_tp("2024-06-01T12:34:56+09:60")]
    #[case::invalid_tp("2024-06-01T12:34:56+24:00")]
    #[case::invalid_tp("2024-06-01T12:34:56-15:60")]
    #[case::invalid_tp("2024-06-01T12:34:56-24:00")]
    #[case::invalid_tp("2024-06-01T12:34:56 Z")]
    #[case::invalid_tp("2024-06-01T12:34:56 09")]
    #[case::invalid_tp("2024-06-01T12:34:56 09:00")]
    #[case::invalid_tp("2024-06-01T12:34:56 09:00:00")]
    #[case::invalid_tp("2024-06-01T12:34:56 +09:00")]
    #[case::invalid_tp("2024-06-01T12:34:56 -09:00")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[]")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[NOT_EXIST]")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[asia/tokyo]")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[Asia/Tokyo ]")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[ Asia/Tokyo]")]
    #[case::invalid_tz("2024-06-01T12:34:56+09:00[ Asia/Tokyo ]")]
    #[case::non_trimmed("2024-06-01T12:34:56+09:00 ")]
    #[case::non_trimmed(" 2024-06-01T12:34:56+09:00")]
    #[case::non_trimmed(" 2024-06-01T12:34:56+09:00 ")]
    fn test_parse_err(#[case] s: &str) {
        let tested = DateTime::from_str(s);

        assert!(tested.is_err());
    }

    #[rstest]
    #[case("1970-01-01T00:00:00Z")]
    #[case("2024-06-01T12:34:56Z")]
    #[case("2999-12-31T23:59:59Z")]
    #[case("1970-01-01T00:00:00+09:00")]
    #[case("2024-06-01T12:34:56+09:00")]
    #[case("2999-12-31T23:59:59+09:00")]
    #[case("1970-01-01T00:00:00-05:23")]
    #[case("2024-06-01T12:34:56-05:23")]
    #[case("2999-12-31T23:59:59-05:23")]
    #[case("1970-01-01T00:00:00+00:00[UTC]")]
    #[case("2024-06-01T12:34:56-04:00[America/New_York]")]
    #[case("2999-12-31T23:59:59+09:00[Asia/Tokyo]")]
    fn test_to_string(#[case] s: &str) {
        let dt = DateTime::from_str(s).unwrap();
        let tested = dt.to_string();

        assert_eq!(tested, s);
    }
}
