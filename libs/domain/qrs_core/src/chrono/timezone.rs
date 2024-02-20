use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use anyhow::{anyhow, ensure, Context};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// TimeZoneOffset
//
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TimeZoneOffset(_TimeZoneOffset);

//
// display, serde
//
impl Debug for TimeZoneOffset {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            _TimeZoneOffset::FixedOffset(offset) => Debug::fmt(&offset, f),
            _TimeZoneOffset::Iana(tz) => Debug::fmt(&tz, f),
        }
    }
}

impl Display for TimeZoneOffset {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            _TimeZoneOffset::FixedOffset(offset) => write!(f, "{}", offset),
            _TimeZoneOffset::Iana(tz) => write!(f, "{}", tz),
        }
    }
}

//
// methods
//
impl chrono::Offset for TimeZoneOffset {
    #[inline]
    fn fix(&self) -> chrono::FixedOffset {
        match self.0 {
            _TimeZoneOffset::FixedOffset(offset) => offset,
            _TimeZoneOffset::Iana(tz) => tz.fix(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum _TimeZoneOffset {
    FixedOffset(chrono::FixedOffset),
    Iana(<chrono_tz::Tz as chrono::TimeZone>::Offset),
}

// -----------------------------------------------------------------------------
// Timezone
//
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeZone {
    FixedOffset(chrono::FixedOffset),
    Iana(chrono_tz::Tz),
}

//
// display, serde
//
impl Debug for TimeZone {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeZone::FixedOffset(offset) => Debug::fmt(offset, f),
            TimeZone::Iana(tz) => Debug::fmt(tz, f),
        }
    }
}

impl Display for TimeZone {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeZone::FixedOffset(offset) => write!(f, "{}", offset),
            TimeZone::Iana(tz) => write!(f, "{}", tz),
        }
    }
}

impl JsonSchema for TimeZone {
    fn schema_name() -> String {
        "TimeZone".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_core::chrono::TimeZone".into()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        res.metadata().description = Some(
            "String for timezone. Either of '+/-HH:mm', '+/-HH:mm:ss' or IANA timezone identifier is available"
                .to_string(),
        );
        res.metadata().examples = vec![
            "+09:00".to_string().into(),
            "-05:00".to_string().into(),
            "+09:00:00".to_string().into(),
            "Asia/Tokyo".to_string().into(),
            "America/New_York".to_string().into(),
            "Etc/UTC".to_string().into(),
        ];
        res.into()
    }
}

impl Serialize for TimeZone {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TimeZone::FixedOffset(offset) => serializer.serialize_str(&offset.to_string()),
            TimeZone::Iana(tz) => tz.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TimeZone {
    fn deserialize<D>(deserializer: D) -> Result<TimeZone, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TimeZone::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for TimeZone {
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let non_iana_reason = match chrono_tz::Tz::from_str(s) {
            Ok(tz) => return Ok(TimeZone::Iana(tz)),
            Err(e) => e,
        };
        let non_fixed_offset_reason = match _parse_fixed_offset(s) {
            Ok(offset) => return Ok(TimeZone::FixedOffset(offset)),
            Err(e) => e,
        };
        Err(anyhow!(
            "Invalid timezone. non_iana_reason=[{}]. non_fixedoffset_reason=[{}].",
            non_iana_reason,
            non_fixed_offset_reason
        ))
    }
}

fn _parse_fixed_offset(s: &str) -> Result<chrono::FixedOffset, anyhow::Error> {
    if s.is_empty() {
        return Err(anyhow::anyhow!("Invalid offset. Offset must not be empty"));
    }
    let (sign, time) = s.split_at(1);
    let sign = match sign {
        "+" => 1,
        "-" => -1,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid offset. Offset must start with '+' or '-'"
            ))
        }
    };
    let (hour, minsec) = match time.split_once(':') {
        Some((hour, minsec)) => {
            ensure!(hour.len() == 2, "Invalid hour. Hour must be in 2 digits");
            let hour = hour.parse::<i32>().context("Invalid hour")?;
            (hour, minsec)
        }
        None => {
            return Err(anyhow::anyhow!(
                "Invalid offset. Offset must be in '+/-HH:MM' or '+/-HH:MM:SS' format"
            ))
        }
    };
    if !(0..24).contains(&hour) {
        return Err(anyhow::anyhow!("Invalid hour. Hour must be in 0-23"));
    }
    let (minute, seconds) = match minsec.split_once(':') {
        Some((minute, seconds)) => {
            ensure!(
                minute.len() == 2,
                "Invalid minute. Minute must be in 2 digits"
            );
            ensure!(
                seconds.len() == 2,
                "Invalid seconds. Seconds must be in 2 digits"
            );
            let minute = minute.parse::<i32>().context("Invalid minute")?;
            let seconds = seconds.parse::<i32>().context("Invalid seconds")?;
            (minute, seconds)
        }
        None => {
            ensure!(
                minsec.len() == 2,
                "Invalid minute. Minute must be in 2 digits"
            );
            let minute = minsec.parse::<i32>().context("Invalid minute")?;
            (minute, 0)
        }
    };
    if !(0..60).contains(&minute) {
        return Err(anyhow::anyhow!("Invalid minute. Minute must be in 0-59"));
    }
    if !(0..60).contains(&seconds) {
        return Err(anyhow::anyhow!("Invalid seconds. Seconds must be in 0-59"));
    }
    chrono::FixedOffset::east_opt(sign * (hour * 3600 + minute * 60 + seconds)).ok_or_else(|| {
        anyhow::anyhow!("Invalid offset. Offset must be in '+/-HH:MM' or '+/-HH:MM:SS' format")
    })
}

//
// construction
//
impl Default for TimeZone {
    #[inline]
    fn default() -> Self {
        TimeZone::FixedOffset(chrono::FixedOffset::east_opt(0).unwrap())
    }
}

impl From<chrono::FixedOffset> for TimeZone {
    #[inline]
    fn from(offset: chrono::FixedOffset) -> Self {
        TimeZone::FixedOffset(offset)
    }
}

impl From<chrono_tz::Tz> for TimeZone {
    #[inline]
    fn from(tz: chrono_tz::Tz) -> Self {
        TimeZone::Iana(tz)
    }
}

//
// methods
//
impl chrono::TimeZone for TimeZone {
    type Offset = TimeZoneOffset;

    #[inline]
    fn from_offset(offset: &Self::Offset) -> Self {
        match offset.0 {
            _TimeZoneOffset::FixedOffset(offset) => TimeZone::FixedOffset(offset),
            _TimeZoneOffset::Iana(offset) => TimeZone::Iana(chrono_tz::Tz::from_offset(&offset)),
        }
    }

    #[inline]
    fn offset_from_local_date(
        &self,
        local: &chrono::prelude::NaiveDate,
    ) -> chrono::LocalResult<Self::Offset> {
        match self {
            TimeZone::FixedOffset(offset) => offset
                .offset_from_local_date(local)
                .map(|offset| TimeZoneOffset(_TimeZoneOffset::FixedOffset(offset))),
            TimeZone::Iana(tz) => tz
                .offset_from_local_date(local)
                .map(|offset| TimeZoneOffset(_TimeZoneOffset::Iana(offset))),
        }
    }

    #[inline]
    fn offset_from_local_datetime(
        &self,
        local: &chrono::prelude::NaiveDateTime,
    ) -> chrono::LocalResult<Self::Offset> {
        match self {
            TimeZone::FixedOffset(offset) => offset
                .offset_from_local_datetime(local)
                .map(|offset| TimeZoneOffset(_TimeZoneOffset::FixedOffset(offset))),
            TimeZone::Iana(tz) => tz
                .offset_from_local_datetime(local)
                .map(|offset| TimeZoneOffset(_TimeZoneOffset::Iana(offset))),
        }
    }

    #[inline]
    fn offset_from_utc_date(&self, utc: &chrono::prelude::NaiveDate) -> Self::Offset {
        match self {
            TimeZone::FixedOffset(offset) => TimeZoneOffset(_TimeZoneOffset::FixedOffset(
                offset.offset_from_utc_date(utc),
            )),
            TimeZone::Iana(tz) => {
                TimeZoneOffset(_TimeZoneOffset::Iana(tz.offset_from_utc_date(utc)))
            }
        }
    }

    #[inline]
    fn offset_from_utc_datetime(&self, utc: &chrono::prelude::NaiveDateTime) -> Self::Offset {
        match self {
            TimeZone::FixedOffset(offset) => TimeZoneOffset(_TimeZoneOffset::FixedOffset(
                offset.offset_from_utc_datetime(utc),
            )),
            TimeZone::Iana(tz) => {
                TimeZoneOffset(_TimeZoneOffset::Iana(tz.offset_from_utc_datetime(utc)))
            }
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::chrono::{timezone::TimeZoneOffset, TimeZone};

    #[test]
    fn test_display() {
        use super::*;
        assert_eq!(
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600).unwrap()).to_string(),
            "+09:00"
        );
        assert_eq!(
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60).unwrap())
                .to_string(),
            "+09:30"
        );
        assert_eq!(
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60 + 15).unwrap())
                .to_string(),
            "+09:30:15"
        );
        assert_eq!(
            TimeZone::Iana(chrono_tz::Tz::Asia__Tokyo).to_string(),
            "Asia/Tokyo"
        );
        assert_eq!(
            TimeZone::Iana(chrono_tz::Tz::America__New_York).to_string(),
            "America/New_York"
        );
        assert_eq!(
            TimeZone::Iana(chrono_tz::Tz::Etc__UTC).to_string(),
            "Etc/UTC"
        );
    }

    #[test]
    fn test_serialize() {
        use serde_json::json;
        assert_eq!(
            serde_json::to_value(TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(9 * 3600).unwrap()
            ))
            .unwrap(),
            json!("+09:00")
        );
        assert_eq!(
            serde_json::to_value(TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60).unwrap()
            ))
            .unwrap(),
            json!("+09:30")
        );
        assert_eq!(
            serde_json::to_value(TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60 + 15).unwrap()
            ))
            .unwrap(),
            json!("+09:30:15")
        );
        assert_eq!(
            serde_json::to_value(TimeZone::Iana(chrono_tz::Tz::Asia__Tokyo)).unwrap(),
            json!("Asia/Tokyo")
        );
        assert_eq!(
            serde_json::to_value(TimeZone::Iana(chrono_tz::Tz::America__New_York)).unwrap(),
            json!("America/New_York")
        );
        assert_eq!(
            serde_json::to_value(TimeZone::Iana(chrono_tz::Tz::Etc__UTC)).unwrap(),
            json!("Etc/UTC")
        );
    }

    #[test]
    fn test_deserialize() {
        use serde_json::json;
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("+09:00")).unwrap(),
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600).unwrap())
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("+09:30")).unwrap(),
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60).unwrap())
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("+09:30:15")).unwrap(),
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60 + 15).unwrap())
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("-09:30")).unwrap(),
            TimeZone::FixedOffset(chrono::FixedOffset::east_opt(-(9 * 3600 + 30 * 60)).unwrap())
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("Asia/Tokyo")).unwrap(),
            TimeZone::Iana(chrono_tz::Tz::Asia__Tokyo)
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("America/New_York")).unwrap(),
            TimeZone::Iana(chrono_tz::Tz::America__New_York)
        );
        assert_eq!(
            serde_json::from_value::<TimeZone>(json!("Etc/UTC")).unwrap(),
            TimeZone::Iana(chrono_tz::Tz::Etc__UTC)
        );

        // error
        assert!(serde_json::from_value::<TimeZone>(json!("+09:")).is_err());
        assert!(serde_json::from_value::<TimeZone>(json!("+09:30:")).is_err());
        assert!(serde_json::from_value::<TimeZone>(json!("+09:30:15:00")).is_err());
        assert!(serde_json::from_value::<TimeZone>(json!("+09:30:15:00")).is_err());
        assert!(serde_json::from_value::<TimeZone>(json!("")).is_err());
    }

    #[test]
    fn test_from_utc_datetime() {
        use chrono::prelude::*;

        // FixedOffset
        let internal = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let tz = super::TimeZone::FixedOffset(internal);
        let naive =
            NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let with_tz = tz.from_utc_datetime(&naive);
        let with_internal = internal.from_utc_datetime(&naive);
        assert_eq!(with_tz, with_internal);

        // Iana
        let internal = chrono_tz::Tz::Asia__Tokyo;
        let tz = super::TimeZone::Iana(internal);
        let naive =
            NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let with_tz = tz.from_utc_datetime(&naive);
        let with_internal = internal.from_utc_datetime(&naive);
        assert_eq!(with_tz, with_internal);
    }

    #[test]
    #[allow(deprecated)]
    fn test_from_utc_date() {
        use chrono::prelude::*;

        // FixedOffset
        let internal = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let tz = super::TimeZone::FixedOffset(internal);
        let naive = NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d").unwrap();
        let with_tz = tz.from_utc_date(&naive);
        let with_internal = internal.from_utc_date(&naive);
        assert_eq!(with_tz, with_internal);

        // Iana
        let internal = chrono_tz::Tz::Asia__Tokyo;
        let tz = super::TimeZone::Iana(internal);
        let naive = NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d").unwrap();
        let with_tz = tz.from_utc_date(&naive);
        let with_internal = internal.from_utc_date(&naive);
        assert_eq!(with_tz, with_internal);
    }

    #[test]
    fn test_offset_from_utc_datetime() {
        use chrono::prelude::*;

        // FixedOffset
        let internal = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let tz = super::TimeZone::FixedOffset(internal);
        let naive =
            NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let with_tz = tz.offset_from_utc_datetime(&naive);
        let with_internal = internal.offset_from_utc_datetime(&naive);
        assert_eq!(
            with_tz,
            TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(with_internal))
        );

        // Iana
        let internal = chrono_tz::Tz::Asia__Tokyo;
        let tz = super::TimeZone::Iana(internal);
        let naive =
            NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let with_tz = tz.offset_from_utc_datetime(&naive);
        let with_internal = internal.offset_from_utc_datetime(&naive);
        assert_eq!(
            with_tz,
            TimeZoneOffset(super::_TimeZoneOffset::Iana(with_internal))
        );
    }

    #[test]
    fn test_offset_from_utc_date() {
        use chrono::prelude::*;

        // FixedOffset
        let internal = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let tz = super::TimeZone::FixedOffset(internal);
        let naive = NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d").unwrap();
        let with_tz = tz.offset_from_utc_date(&naive);
        let with_internal = internal.offset_from_utc_date(&naive);
        assert_eq!(
            with_tz,
            TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(with_internal))
        );

        // Iana
        let internal = chrono_tz::Tz::Asia__Tokyo;
        let tz = super::TimeZone::Iana(internal);
        let naive = NaiveDate::parse_from_str("2021-01-01", "%Y-%m-%d").unwrap();
        let with_tz = tz.offset_from_utc_date(&naive);
        let with_internal = internal.offset_from_utc_date(&naive);
        assert_eq!(
            with_tz,
            TimeZoneOffset(super::_TimeZoneOffset::Iana(with_internal))
        );
    }

    #[test]
    fn test_fix() {
        use chrono::prelude::*;

        // FixedOffset
        let internal = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal);

        // more cases
        let internal = chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60).unwrap();
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal);

        let internal = chrono::FixedOffset::east_opt(9 * 3600 + 30 * 60 + 15).unwrap();
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal);

        let internal = chrono::FixedOffset::east_opt(-(9 * 3600 + 30 * 60)).unwrap();
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::FixedOffset(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal);

        // Iana
        let internal = chrono_tz::Tz::Asia__Tokyo.offset_from_utc_datetime(
            &NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        );
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::Iana(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal.fix());

        let internal = chrono_tz::Tz::America__New_York.offset_from_utc_datetime(
            &NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        );
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::Iana(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal.fix());

        let internal = chrono_tz::Tz::Etc__UTC.offset_from_utc_datetime(
            &NaiveDateTime::parse_from_str("2021-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        );
        let tz = super::TimeZoneOffset(super::_TimeZoneOffset::Iana(internal));
        let fixed = tz.fix();
        assert_eq!(fixed, internal.fix());
    }
}
