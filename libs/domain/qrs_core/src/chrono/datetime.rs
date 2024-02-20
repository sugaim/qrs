use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::Sub,
    str::FromStr,
};

use anyhow::anyhow;
use chrono::{format::DelayedFormat, TimeZone};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::num::RelPos;

// -----------------------------------------------------------------------------
// DateTime
//
pub type DateTime = GenericDateTime<super::TimeZone>;

// -----------------------------------------------------------------------------
// GenericDateTime
//

/// Thin wrapper around [chrono::DateTime] to override some traits
#[derive(Clone, Hash)]
pub struct GenericDateTime<Tz: chrono::TimeZone> {
    internal: chrono::DateTime<Tz>,
}

impl<Tz: TimeZone> Copy for GenericDateTime<Tz> where Tz::Offset: Copy {}

//
// display, serde
//
impl<Tz: TimeZone> Debug for GenericDateTime<Tz> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.internal, f)
    }
}

impl<Tz: TimeZone> Display for GenericDateTime<Tz>
where
    Tz::Offset: Display,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.internal, f)
    }
}

impl Serialize for GenericDateTime<chrono::FixedOffset> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.internal.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GenericDateTime<chrono::FixedOffset> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        chrono::DateTime::deserialize(deserializer).map(Into::into)
    }
}

impl JsonSchema for GenericDateTime<chrono::FixedOffset> {
    fn schema_name() -> String {
        "DateTimeFixedOffset".to_string()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("qrs_core::chrono::DateTimeFixedOffset")
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res =
            <chrono::DateTime<chrono::FixedOffset> as JsonSchema>::json_schema(gen).into_object();
        res.metadata().description = Some("A datetime with fixed offset timezone".to_string());
        res.into()
    }
}

impl Serialize for GenericDateTime<chrono::Utc> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.internal.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GenericDateTime<chrono::Utc> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        chrono::DateTime::deserialize(deserializer).map(Into::into)
    }
}

impl JsonSchema for GenericDateTime<chrono::Utc> {
    fn schema_name() -> String {
        "DateTimeUtc".to_string()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("qrs_core::chrono::DateTimeUtc")
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = <chrono::DateTime<chrono::Utc> as JsonSchema>::json_schema(gen).into_object();
        res.metadata().description = Some("A datetime with UTC timezone".to_string());
        res.into()
    }
}

impl Serialize for GenericDateTime<chrono_tz::Tz> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!(
            "{}[{}]",
            self.internal.to_rfc3339(),
            self.internal.timezone()
        );
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for GenericDateTime<chrono_tz::Tz> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        GenericDateTime::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for GenericDateTime<chrono_tz::Tz> {
    fn schema_name() -> String {
        "DateTimeIana".to_string()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("qrs_core::chrono::DateTimeIana")
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        res.metadata().description =
            Some("A datetime with IANA timezone, {RFC3339}[{IANA timezone}] or %Y-%m-%dT%H:%M:%S[{IANA timezone}]".to_string());
        res.metadata().examples = vec![
            serde_json::json!("2021-01-01T10:42:11[America/New_York]"),
            serde_json::json!("2021-01-01T10:42:11Z[America/New_York]"),
            serde_json::json!("2021-01-01T10:42:11+09:00[Asia/Tokyo]"),
        ];

        res.string().pattern = Some(
            r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[\+-]\d{2}:\d{2}|[\+-]\d{4})?(\[.+\])$"
                .to_owned(),
        );
        res.into()
    }
}

impl Serialize for GenericDateTime<super::TimeZone> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.internal.timezone() {
            super::TimeZone::FixedOffset(tz) => self.with_timezone(&tz).serialize(serializer),
            super::TimeZone::Iana(tz) => self.with_timezone(&tz).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for GenericDateTime<super::TimeZone> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Dt {
            FixedOffset(GenericDateTime<chrono::FixedOffset>),
            Iana(GenericDateTime<chrono_tz::Tz>),
        }
        let dt = Dt::deserialize(deserializer)?;
        Ok(match dt {
            Dt::FixedOffset(dt) => dt.with_timezone(&dt.internal.timezone().into()),
            Dt::Iana(dt) => dt.with_timezone(&dt.internal.timezone().into()),
        })
    }
}

impl JsonSchema for GenericDateTime<super::TimeZone> {
    fn schema_name() -> String {
        "DateTime".to_string()
    }
    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("qrs_core::chrono::DateTime")
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject::default();
        res.subschemas().one_of = Some(vec![
            gen.subschema_for::<GenericDateTime<chrono::FixedOffset>>(),
            gen.subschema_for::<GenericDateTime<chrono_tz::Tz>>(),
        ]);
        res.metadata().description =
            Some("A datetime with fixed offset or IANA timezone".to_string());
        res.metadata().examples = vec![
            serde_json::json!("2021-01-01T10:42:11+09:00"),
            serde_json::json!("2021-01-01T10:42:11Z"),
            serde_json::json!("2021-01-01T10:42:11[America/New_York]"),
            serde_json::json!("2021-01-01T10:42:11Z[America/New_York]"),
            serde_json::json!("2021-01-01T10:42:11+09:00[Asia/Tokyo]"),
        ];
        res.into()
    }
}

impl<Tz: TimeZone> GenericDateTime<Tz> {
    #[inline]
    pub fn to_rfc3339(&self) -> String {
        self.internal.to_rfc3339()
    }
}

impl<Tz: TimeZone> GenericDateTime<Tz>
where
    Tz::Offset: Display,
{
    /// Formats the combined date and time with the specified format string.
    /// See [chrono::DateTime::format] for more details.
    ///
    /// # Examples
    /// ```
    /// use qrs_core::chrono::DateTime;
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
// comparison
//
impl<Tz1, Tz2> PartialEq<GenericDateTime<Tz2>> for GenericDateTime<Tz1>
where
    Tz1: TimeZone,
    Tz2: TimeZone,
{
    #[inline]
    fn eq(&self, other: &GenericDateTime<Tz2>) -> bool {
        self.internal.eq(&other.internal)
    }
}

impl<Tz: TimeZone> Eq for GenericDateTime<Tz> {}

impl<Tz1, Tz2> PartialOrd<GenericDateTime<Tz2>> for GenericDateTime<Tz1>
where
    Tz1: TimeZone,
    Tz2: TimeZone,
{
    #[inline]
    fn partial_cmp(&self, other: &GenericDateTime<Tz2>) -> Option<std::cmp::Ordering> {
        self.internal.partial_cmp(&other.internal)
    }
}

impl<Tz: TimeZone> Ord for GenericDateTime<Tz> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.internal.cmp(&other.internal)
    }
}

//
// construction
//
impl<Tz: TimeZone> From<chrono::DateTime<Tz>> for GenericDateTime<Tz> {
    #[inline]
    fn from(internal: chrono::DateTime<Tz>) -> Self {
        Self { internal }
    }
}

impl<Tz: TimeZone> From<GenericDateTime<Tz>> for chrono::DateTime<Tz> {
    #[inline]
    fn from(dt: GenericDateTime<Tz>) -> Self {
        dt.internal
    }
}

impl<Tz: TimeZone> GenericDateTime<Tz> {
    #[inline]
    pub fn new(datetime: chrono::NaiveDateTime, tz: Tz) -> Self {
        tz.from_local_datetime(&datetime).single().unwrap().into()
    }
}

impl FromStr for GenericDateTime<chrono::FixedOffset> {
    type Err = chrono::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        chrono::DateTime::from_str(s).map(|dt| dt.into())
    }
}

impl FromStr for GenericDateTime<chrono::Utc> {
    type Err = chrono::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        chrono::DateTime::from_str(s).map(|dt| dt.into())
    }
}

impl FromStr for GenericDateTime<chrono_tz::Tz> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (dt, tz) = {
            let mut split = s.split('[');
            let dt = split.next().ok_or_else(|| {
                anyhow!(
                    "Failed to parse datetime from string. Expected format is
                    '{{RFC3339}}[{{IANA timezone}}]' but was '{s}'"
                )
            })?;
            let tz = split.next().ok_or_else(|| {
                anyhow!(
                    "Failed to parse timezone from string
                    '{{RFC3339}}[{{IANA timezone}}]' but was '{s}'"
                )
            })?;
            if !tz.ends_with(']') {
                return Err(anyhow!(
                    "Failed to parse timezone from string
                    '{{RFC3339}}[{{IANA timezone}}]' but was '{s}'"
                ));
            }
            (dt, &tz[..tz.len() - 1])
        };
        let tz = chrono_tz::Tz::from_str(tz)
            .map_err(|_| anyhow!("Invalid IANA timezone string: {tz}"))?;
        if let Ok(dttz) = chrono::DateTime::<chrono::FixedOffset>::from_str(dt) {
            return Ok(dttz.with_timezone(&tz).into());
        }
        if let Ok(dt) = chrono::NaiveDateTime::from_str(dt) {
            return match dt.and_local_timezone(tz) {
                chrono::LocalResult::Single(dt) => Ok(dt.into()),
                _ => Err(anyhow!("Failed to parse datetime from string: {s}")),
            };
        }
        Err(anyhow!(
            "Failed to parse datetime from string. Expected format is
            '{{RFC3339}}[{{IANA timezone}}]' or '{{RFC3339}}[{{FixedOffset timezone}}]' but was '{s}'"
        ))
    }
}

impl FromStr for GenericDateTime<super::TimeZone> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(dt) = GenericDateTime::<chrono_tz::Tz>::from_str(s) {
            return Ok(dt.with_timezone(&super::TimeZone::Iana(dt.internal.timezone())));
        }
        if let Ok(dt) = GenericDateTime::<chrono::FixedOffset>::from_str(s) {
            return Ok(dt.with_timezone(&super::TimeZone::FixedOffset(*dt.internal.offset())));
        }
        Err(anyhow!(
            "Failed to parse datetime from string. Expected format is
            '{{RFC3339}}[{{IANA timezone}}]' or '{{RFC3339}}[{{FixedOffset timezone}}]' but was '{s}'"
        ))
    }
}

//
// getters
//
impl<Tz: TimeZone> chrono::Datelike for GenericDateTime<Tz> {
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

impl<Tz: TimeZone> chrono::Timelike for GenericDateTime<Tz> {
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

impl<Tz: TimeZone> GenericDateTime<Tz> {
    /// Returns a reference to the underlying [chrono::DateTime] object.
    ///
    /// # Examples
    /// ```
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
    /// use std::str::FromStr;
    ///
    /// let dt = DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
    /// let dt_utc = dt.with_timezone(&chrono::Utc);
    ///
    /// assert_eq!(dt_utc, chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T01:42:11Z").unwrap().into());
    /// ```
    #[inline]
    pub fn with_timezone<U: TimeZone>(&self, tz: &U) -> GenericDateTime<U> {
        self.internal.with_timezone(tz).into()
    }

    /// Same as [chrono::DateTime::timestamp]
    ///
    /// # Examples
    /// ```
    /// use qrs_core::chrono::DateTime;
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
    /// use qrs_core::chrono::DateTime;
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
impl<Tz: TimeZone> Sub for GenericDateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        (self.internal - rhs.internal).into()
    }
}
impl<Tz: TimeZone + Clone> Sub<&GenericDateTime<Tz>> for GenericDateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: &Self) -> Self::Output {
        (self.internal - rhs.internal.clone()).into()
    }
}
impl<Tz: TimeZone + Clone> Sub<GenericDateTime<Tz>> for &GenericDateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: GenericDateTime<Tz>) -> Self::Output {
        (self.internal.clone() - rhs.internal).into()
    }
}
impl<Tz: TimeZone + Clone> Sub<&GenericDateTime<Tz>> for &GenericDateTime<Tz> {
    type Output = super::Duration;

    #[inline]
    fn sub(self, rhs: &GenericDateTime<Tz>) -> Self::Output {
        (self.internal.clone() - rhs.internal.clone()).into()
    }
}

macro_rules! define_self_duration_op {
    ($op:ident, $op_fn:ident) => {
        impl<Tz: TimeZone> std::ops::$op<super::Duration> for GenericDateTime<Tz> {
            type Output = GenericDateTime<Tz>;

            #[inline]
            fn $op_fn(self, rhs: super::Duration) -> Self::Output {
                (self.internal.$op_fn(*rhs.as_chrono())).into()
            }
        }
        impl<Tz: TimeZone> std::ops::$op<&super::Duration> for GenericDateTime<Tz> {
            type Output = GenericDateTime<Tz>;

            #[inline]
            fn $op_fn(self, rhs: &super::Duration) -> Self::Output {
                (self.internal.$op_fn(*rhs.as_chrono())).into()
            }
        }
    };
}
define_self_duration_op!(Add, add);
define_self_duration_op!(Sub, sub);

impl<Tz1, Tz2> RelPos<GenericDateTime<Tz2>> for GenericDateTime<Tz1>
where
    Tz1: TimeZone,
    Tz2: TimeZone,
{
    type Output = f64;

    #[inline]
    fn relpos_between(
        &self,
        left: &GenericDateTime<Tz2>,
        right: &GenericDateTime<Tz2>,
    ) -> Self::Output {
        self.internal
            .relpos_between(&left.internal, &right.internal)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};
    use num::Zero;

    use crate::chrono::TimeZone;

    use super::super::Duration;
    use super::*;

    #[test]
    fn test_debug() {
        // fixed offset
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T10:42:11+09:00");

        // utc
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T10:42:11Z");

        // IANA
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T05:42:11EST");

        // qrs_core::chrono::TimeZone
        let dt =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt: DateTime = dt
            .with_timezone(&TimeZone::FixedOffset(dt.timezone()))
            .into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T10:42:11+09:00");

        let dt = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        let dt: DateTime = dt.with_timezone(&TimeZone::Iana(dt.timezone())).into();
        assert_eq!(format!("{:?}", dt), "2021-01-01T05:42:11EST");
    }

    #[test]
    fn test_display() {
        // fixed offset
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();

        assert_eq!(dt.to_string(), "2021-01-01 10:42:11 +09:00");

        // utc
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();

        assert_eq!(dt.to_string(), "2021-01-01 10:42:11 UTC");

        // IANA
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();

        assert_eq!(dt.to_string(), "2021-01-01 05:42:11 EST");

        // qrs_core::chrono::TimeZone
        let dt =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt: DateTime = dt
            .with_timezone(&TimeZone::FixedOffset(dt.timezone()))
            .into();
        assert_eq!(dt.to_string(), "2021-01-01 10:42:11 +09:00");

        let dt = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        let dt: DateTime = dt.with_timezone(&TimeZone::Iana(dt.timezone())).into();
        assert_eq!(dt.to_string(), "2021-01-01 05:42:11 EST");
    }

    #[test]
    fn test_from_str() {
        // fixed offset
        let dt: GenericDateTime<_> =
            GenericDateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(dt, expected);

        // utc
        let dt: GenericDateTime<_> =
            GenericDateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(dt, expected);

        // IANA
        let dt: GenericDateTime<_> = GenericDateTime::<chrono_tz::Tz>::from_str(
            "2021-01-01T05:42:11-05:00[America/New_York]",
        )
        .unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt, expected);

        let dt: GenericDateTime<_> =
            GenericDateTime::<chrono_tz::Tz>::from_str("2021-01-01T10:42:11Z[America/New_York]")
                .unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt, expected);

        // IANA: error
        let dt: Result<GenericDateTime<_>, _> =
            GenericDateTime::<chrono_tz::Tz>::from_str("2021-01-01T05:42:11-05:00");
        assert!(dt.is_err());

        let dt: Result<GenericDateTime<_>, _> = GenericDateTime::<chrono_tz::Tz>::from_str(
            "2021-01-01T05:42:11-05:00[America/New_York]x",
        );
        assert!(dt.is_err());

        let dt: Result<GenericDateTime<_>, _> = GenericDateTime::<chrono_tz::Tz>::from_str(
            "2021-01-01T05:42:11-05:00x[America/New_York]",
        );
        assert!(dt.is_err());

        let dt: Result<GenericDateTime<_>, _> =
            GenericDateTime::<chrono_tz::Tz>::from_str("2021-01-01T05:42:11-05:00[DUMMY_TIMEZONE]");
        assert!(dt.is_err());

        let dt: Result<GenericDateTime<_>, _> =
            GenericDateTime::<chrono_tz::Tz>::from_str("2021-01-01T05:42:11-05:00[]");
        assert!(dt.is_err());

        // qrs_core::chrono::TimeZone
        let dt = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(dt, expected);

        let dt = DateTime::from_str("2021-01-01T10:42:11Z").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(dt, expected);

        let dt = DateTime::from_str("2021-01-01T05:42:11-05:00").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(dt, expected);

        let dt = DateTime::from_str("2021-01-01T10:42:11Z[America/New_York]").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt, expected);

        let dt = DateTime::from_str("2021-01-01T05:42:11[America/New_York]").unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt, expected);
    }

    #[test]
    fn test_serialize() {
        // fixed offset
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(serialized, r#""2021-01-01T10:42:11+09:00""#);

        // utc
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(serialized, r#""2021-01-01T10:42:11Z""#);

        // IANA
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(
            serialized,
            r#""2021-01-01T05:42:11-05:00[America/New_York]""#
        );

        // qrs_core::chrono::TimeZone
        let dt =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt: DateTime = dt
            .with_timezone(&TimeZone::FixedOffset(dt.timezone()))
            .into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(serialized, r#""2021-01-01T10:42:11+09:00""#);

        let dt = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        let dt: DateTime = dt.with_timezone(&TimeZone::Iana(dt.timezone())).into();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert_eq!(
            serialized,
            r#""2021-01-01T05:42:11-05:00[America/New_York]""#
        );
    }

    #[test]
    fn test_deserialize() {
        // fixed offset
        let serialized = r#""2021-01-01T10:42:11+09:00""#;
        let deserialized: GenericDateTime<chrono::FixedOffset> =
            serde_json::from_str(serialized).unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(deserialized, expected);

        // utc
        let serialized = r#""2021-01-01T10:42:11Z""#;
        let deserialized: GenericDateTime<chrono::Utc> = serde_json::from_str(serialized).unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(deserialized, expected);

        // IANA
        let serialized = r#""2021-01-01T05:42:11-05:00[America/New_York]""#;
        let deserialized: GenericDateTime<chrono_tz::Tz> =
            serde_json::from_str(serialized).unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(deserialized, expected);

        // qrs_core::chrono::TimeZone
        let serialized = r#""2021-01-01T10:42:11+09:00""#;
        let deserialized: DateTime = serde_json::from_str(serialized).unwrap();
        let expected: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_new() {
        // fixed offset
        let dt = GenericDateTime::<chrono::FixedOffset>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        assert_eq!(chrono_dt, expected);

        // utc
        let dt = GenericDateTime::<chrono::Utc>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::Utc,
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z").unwrap();
        assert_eq!(chrono_dt, expected);

        // IANA
        let dt = GenericDateTime::<chrono::Utc>::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::Utc,
        )
        .with_timezone(&chrono_tz::Tz::America__New_York);

        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        assert_eq!(chrono_dt, expected);

        // qrs_core::chrono::TimeZone
        let dt = DateTime::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono::FixedOffset::east_opt(9 * 3600).unwrap().into(),
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00").unwrap();
        assert_eq!(chrono_dt, expected);

        let dt = DateTime::new(
            chrono::NaiveDateTime::from_str("2021-01-01T10:42:11").unwrap(),
            chrono_tz::Tz::America__New_York.into(),
        );
        let chrono_dt: chrono::DateTime<_> = dt.into();
        let expected = chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T15:42:11Z")
            .unwrap()
            .with_timezone(&chrono_tz::Tz::America__New_York);
        assert_eq!(chrono_dt, expected);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn test_sub() {
        // fixed offset
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(&dt1 - &dt2, dt1 - dt2);
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);

        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();

        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:12+09:00")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(&dt1 - &dt2, dt1 - dt2);
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);

        // utc
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
                .unwrap()
                .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        // IANA
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt1 - dt2, Duration::zero());
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:12Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));
        assert_eq!(dt1 - &dt2, dt1 - dt2);
        assert_eq!(&dt1 - dt2, dt1 - dt2);
        assert_eq!(&dt1 - &dt2, dt1 - dt2);

        // between summer time and winter time
        let dt1: GenericDateTime<_> = chrono::NaiveDateTime::from_str("2021-03-13T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        let dt2: GenericDateTime<_> = chrono::NaiveDateTime::from_str("2021-03-14T08:30:00")
            .unwrap()
            .and_local_timezone(chrono_tz::Tz::America__New_York)
            .single()
            .unwrap()
            .into();
        assert_eq!(dt2 - dt1, Duration::with_secs(23 * 60 * 60));
        assert_eq!(&dt2 - &dt1, dt2 - dt1);
        assert_eq!(dt2 - &dt1, dt2 - dt1);
        assert_eq!(&dt2 - dt1, dt2 - dt1);

        // qrs_core::chrono::TimeZone
        let dt1 = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt2 = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        assert_eq!(dt1 - dt2, Duration::zero());

        let dt1 = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt2 = DateTime::from_str("2021-01-01T10:42:12+09:00").unwrap();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));

        let dt1 = DateTime::from_str("2021-01-01T10:42:11Z[America/New_York]").unwrap();
        let dt2 = DateTime::from_str("2021-01-01T10:42:11Z[America/New_York]").unwrap();
        assert_eq!(dt1 - dt2, Duration::zero());

        let dt1 = DateTime::from_str("2021-01-01T10:42:11Z[America/New_York]").unwrap();
        let dt2 = DateTime::from_str("2021-01-01T10:42:12Z[America/New_York]").unwrap();
        assert_eq!(dt1 - dt2, Duration::with_secs(-1));

        let dt1 = DateTime::from_str("2021-01-01T10:42:11Z[America/New_York]").unwrap();
        let dt2 = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        assert_eq!(dt1 - dt2, Duration::with_secs(9 * 3600));
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn test_add_sub_duration() {
        // fixed offset
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);
        assert_eq!(dt - duration, dt);
        assert_eq!(dt - &duration, dt);

        let dt: GenericDateTime<_> =
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
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);
        assert_eq!(dt - duration, dt);
        assert_eq!(dt - &duration, dt);

        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
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
        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let duration = Duration::zero();
        assert_eq!(dt + duration, dt);
        assert_eq!(dt + &duration, dt);

        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
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

        let dt: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
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
        let dt: GenericDateTime<_> = chrono::NaiveDateTime::from_str("2021-03-13T08:30:00")
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

        let dt: GenericDateTime<_> = chrono::NaiveDateTime::from_str("2021-03-14T08:30:00")
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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

        // qrs_core::chrono::TimeZone
        let chrono_dts = vec![
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::FixedOffset(
                    chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
                )),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-04-01T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::FixedOffset(
                    chrono::FixedOffset::west_opt(5 * 3600).unwrap(),
                )),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-12-31T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::Iana(
                    chrono_tz::Tz::America__New_York,
                )),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime = chrono_dt.into();
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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
            let dt: GenericDateTime<_> = chrono_dt.into();
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

        // qrs_core::chrono::TimeZone
        let chrono_dts = vec![
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::FixedOffset(
                    chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
                )),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-04-01T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::FixedOffset(
                    chrono::FixedOffset::west_opt(5 * 3600).unwrap(),
                )),
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-12-31T10:42:11+09:00")
                .unwrap()
                .with_timezone(&crate::chrono::TimeZone::Iana(
                    chrono_tz::Tz::America__New_York,
                )),
        ];
        for chrono_dt in chrono_dts {
            let dt: DateTime = chrono_dt.into();
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

    #[test]
    fn test_relpos() {
        // fixed
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-01T10:42:11+09:00")
                .unwrap()
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-06T10:42:12+09:00")
                .unwrap()
                .into();
        let dt3: GenericDateTime<_> =
            chrono::DateTime::<chrono::FixedOffset>::from_str("2021-01-11T10:42:13+09:00")
                .unwrap()
                .into();

        assert_eq!(dt2.relpos_between(&dt1, &dt3), 0.5);

        // utc
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-06T10:42:12Z")
                .unwrap()
                .into();
        let dt3: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-11T10:42:13Z")
                .unwrap()
                .into();

        assert_eq!(dt2.relpos_between(&dt1, &dt3), 0.5);

        // IANA
        let dt1: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-01T10:42:11Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let dt2: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-06T10:42:12Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();
        let dt3: GenericDateTime<_> =
            chrono::DateTime::<chrono::Utc>::from_str("2021-01-11T10:42:13Z")
                .unwrap()
                .with_timezone(&chrono_tz::Tz::America__New_York)
                .into();

        assert_eq!(dt2.relpos_between(&dt1, &dt3), 0.5);

        // qrs_core::chrono::TimeZone
        let dt1 = DateTime::from_str("2021-01-01T10:42:11+09:00").unwrap();
        let dt2 = DateTime::from_str("2021-01-06T10:42:12+09:00").unwrap();
        let dt3 = DateTime::from_str("2021-01-11T10:42:13+09:00").unwrap();

        assert_eq!(dt2.relpos_between(&dt1, &dt3), 0.5);
    }
}
