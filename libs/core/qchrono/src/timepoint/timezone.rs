use std::str::FromStr;

// -----------------------------------------------------------------------------
// TzOffset
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TzOffset {
    Utc(<chrono::Utc as chrono::TimeZone>::Offset),
    FixedOffset(<chrono::FixedOffset as chrono::TimeZone>::Offset),
    IANA(<chrono_tz::Tz as chrono::TimeZone>::Offset),
}

impl chrono::Offset for TzOffset {
    #[inline]
    fn fix(&self) -> chrono::FixedOffset {
        match self {
            TzOffset::Utc(offset) => offset.fix(),
            TzOffset::FixedOffset(offset) => offset.fix(),
            TzOffset::IANA(offset) => offset.fix(),
        }
    }
}

// -----------------------------------------------------------------------------
// Tz
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tz {
    Utc,
    FixedOffset(chrono::FixedOffset),
    Iana(chrono_tz::Tz),
}

//
// conversion
//
impl From<chrono::Utc> for Tz {
    #[inline]
    fn from(_: chrono::Utc) -> Self {
        Tz::Utc
    }
}
impl From<chrono::FixedOffset> for Tz {
    #[inline]
    fn from(offset: chrono::FixedOffset) -> Self {
        Tz::FixedOffset(offset)
    }
}
impl From<chrono_tz::Tz> for Tz {
    #[inline]
    fn from(tz: chrono_tz::Tz) -> Self {
        Tz::Iana(tz)
    }
}

impl chrono::TimeZone for Tz {
    type Offset = TzOffset;

    #[inline]
    fn from_offset(offset: &Self::Offset) -> Self {
        match offset {
            TzOffset::Utc(offset) => chrono::Utc::from_offset(offset).into(),
            TzOffset::FixedOffset(offset) => chrono::FixedOffset::from_offset(offset).into(),
            TzOffset::IANA(offset) => chrono_tz::Tz::from_offset(offset).into(),
        }
    }
    #[inline]
    fn offset_from_local_date(
        &self,
        local: &chrono::NaiveDate,
    ) -> chrono::MappedLocalTime<Self::Offset> {
        match self {
            Tz::Utc => chrono::Utc.offset_from_local_date(local).map(TzOffset::Utc),
            Tz::FixedOffset(offset) => offset
                .offset_from_local_date(local)
                .map(TzOffset::FixedOffset),
            Tz::Iana(tz) => tz.offset_from_local_date(local).map(TzOffset::IANA),
        }
    }
    #[inline]
    fn offset_from_local_datetime(
        &self,
        local: &chrono::NaiveDateTime,
    ) -> chrono::MappedLocalTime<Self::Offset> {
        match self {
            Tz::Utc => chrono::Utc
                .offset_from_local_datetime(local)
                .map(TzOffset::Utc),
            Tz::FixedOffset(offset) => offset
                .offset_from_local_datetime(local)
                .map(TzOffset::FixedOffset),
            Tz::Iana(tz) => tz.offset_from_local_datetime(local).map(TzOffset::IANA),
        }
    }
    #[inline]
    fn offset_from_utc_date(&self, utc: &chrono::NaiveDate) -> Self::Offset {
        match self {
            Tz::Utc => TzOffset::Utc(chrono::Utc.offset_from_utc_date(utc)),
            Tz::FixedOffset(offset) => TzOffset::FixedOffset(offset.offset_from_utc_date(utc)),
            Tz::Iana(tz) => TzOffset::IANA(tz.offset_from_utc_date(utc)),
        }
    }
    #[inline]
    fn offset_from_utc_datetime(&self, utc: &chrono::NaiveDateTime) -> Self::Offset {
        match self {
            Tz::Utc => TzOffset::Utc(chrono::Utc.offset_from_utc_datetime(utc)),
            Tz::FixedOffset(offset) => TzOffset::FixedOffset(offset.offset_from_utc_datetime(utc)),
            Tz::Iana(tz) => TzOffset::IANA(tz.offset_from_utc_datetime(utc)),
        }
    }
}

//
// ser/de
//
impl FromStr for Tz {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s != s.trim() {
            anyhow::bail!("Non-trimmed timezone string({})", s);
        }
        if s == "Z" {
            return Ok(Tz::Utc);
        }
        if let Ok(tz) = chrono::FixedOffset::from_str(s) {
            return Ok(Tz::FixedOffset(tz));
        }
        if let Ok(tz) = chrono_tz::Tz::from_str(s) {
            return Ok(Tz::Iana(tz));
        }
        anyhow::bail!(
            "Invalid timezone({}). Only Z, fixed offset or IANA timezone strings are supported",
            s
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test() {}

    #[rstest]
    #[case::ok("Z", Some(Tz::Utc))]
    #[case::ok("+09:00", chrono::FixedOffset::east_opt(9 * 3600).map(Into::into))]
    #[case::ok("+09:59", chrono::FixedOffset::east_opt(9 * 3600 + 59 * 60).map(Into::into))]
    #[case::ok("+23:59", chrono::FixedOffset::east_opt(23 * 3600 + 59 * 60).map(Into::into))]
    #[case::ok("-15:00", chrono::FixedOffset::west_opt(15 * 3600).map(Into::into))]
    #[case::ok("-15:59", chrono::FixedOffset::west_opt(15 * 3600 + 59 * 60).map(Into::into))]
    #[case::ok("-23:59", chrono::FixedOffset::west_opt(23 * 3600 + 59 * 60).map(Into::into))]
    #[case::ok("Asia/Tokyo", Some(Into::into(chrono_tz::Tz::Asia__Tokyo)))]
    #[case::ok("UTC", Some(Into::into(chrono_tz::Tz::UTC)))]
    #[case::err_nontrim("Z ", None)]
    #[case::err_nontrim(" Z", None)]
    #[case::err_nontrim(" Z ", None)]
    #[case::err_nontrim("+09:00 ", None)]
    #[case::err_nontrim(" +09:00", None)]
    #[case::err_nontrim(" +09:00 ", None)]
    #[case::err_nontrim("Asia/Tokyo ", None)]
    #[case::err_nontrim(" Asia/Tokyo", None)]
    #[case::err_nontrim(" Asia/Tokyo ", None)]
    #[case::err_invalid_fixed_offset("09:00", None)]
    #[case::err_invalid_fixed_offset("9:00", None)]
    #[case::err_invalid_fixed_offset("09:00", None)]
    #[case::err_invalid_fixed_offset("+09", None)]
    #[case::err_invalid_fixed_offset("+09:60", None)]
    #[case::err_invalid_fixed_offset("+24:00", None)]
    #[case::err_invalid_fixed_offset("-15:60", None)]
    #[case::err_invalid_fixed_offset("-24:00", None)]
    #[case::non_existing("Asia/NonExisting", None)]
    #[case::case_sensitive("asia/tokyo", None)]
    fn test_tz_from_str(#[case] s: &str, #[case] expected: Option<Tz>) {
        let tested = Tz::from_str(s).ok();

        assert_eq!(tested, expected);
    }
}
