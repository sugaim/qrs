use std::{borrow::Borrow, fmt::Display, str::FromStr};

use anyhow::anyhow;
use chrono::NaiveDate;
use qrs_datasrc::DataSrc;

use crate::{DateTime, Tenor, TimeCut};

// -----------------------------------------------------------------------------
// DateWithTagSym
//
/// Date with a time cut tag.
///
/// Actual values of time cut can be varying.
/// For example, if 'tokyo close' means close time at tokyo trading market,
/// the time cut can be changed when the market open time is changed.
///
/// In such cases, introducing a tag to represent the time cut is useful
/// for dayly operations.
///
/// # String representation
/// The string representation of this type is `{{date}}@{{tag}}`.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateWithTag<Tag = String> {
    pub date: NaiveDate,
    pub tag: Tag,
}

//
// display, serde
//
impl<Tag: Display> Display for DateWithTag<Tag> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.date, self.tag)
    }
}

impl<Syn: FromStr> FromStr for DateWithTag<Syn>
where
    anyhow::Error: From<<Syn as FromStr>::Err>,
{
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (date, sym) = s.split_once('@').ok_or_else(|| {
            anyhow!(
                "Fail to parse DateWithTag from string: {}. The string must contain '@' to separate date and symbol",
                s
            )
        })?;
        Ok(DateWithTag {
            date: date
                .parse()
                .map_err(|_| anyhow!("Fail to parse date from string: {}", date))?,
            tag: sym.parse()?,
        })
    }
}

#[cfg(feature = "serde")]
impl<Sym> serde::Serialize for DateWithTag<Sym>
where
    Sym: Display,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{}", self);
        serializer.serialize_str(&s)
    }
}

#[cfg(feature = "serde")]
impl<'de, Sym> serde::Deserialize<'de> for DateWithTag<Sym>
where
    Sym: FromStr,
    anyhow::Error: From<<Sym as FromStr>::Err>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<DateWithTag<Sym>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateWithTag::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<Sym> schemars::JsonSchema for DateWithTag<Sym>
where
    Sym: schemars::JsonSchema + Display + FromStr,
{
    fn schema_name() -> String {
        format!("DateWithTag_for_{}", Sym::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_chrono::DateWithTag<{}>", Sym::schema_id()).into()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        res.metadata().description =
            Some("Date with a time cut tag. Format is 'yyyy-MM-dd@{tag}'".to_owned());
        res.string().pattern = Some("^\\d{4}-\\d{2}-\\d{2}@.*$".to_owned());
        res.into()
    }
}

//
// methods
//
impl<Sym> DateWithTag<Sym> {
    #[inline]
    pub fn to_datetime<Tz, Cut, Res, Rq>(
        &self,
        resolver: &Res,
    ) -> Result<DateTime<Tz>, anyhow::Error>
    where
        Rq: ?Sized,
        Tz: chrono::TimeZone,
        Cut: TimeCut<Tz = Tz>,
        Sym: Borrow<Rq>,
        Res: DataSrc<Rq, Output = Cut>,
        anyhow::Error: From<Cut::Err>,
    {
        let cut = resolver.get(self.tag.borrow())?;
        cut.to_datetime(self.date).map_err(Into::into)
    }
}

//
// operators
//
impl<Sym> std::ops::Add<Tenor> for DateWithTag<Sym> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Tenor) -> Self::Output {
        DateWithTag {
            date: self.date + rhs,
            tag: self.tag,
        }
    }
}

impl<Sym> std::ops::Sub<Tenor> for DateWithTag<Sym> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Tenor) -> Self::Output {
        DateWithTag {
            date: self.date - rhs,
            tag: self.tag,
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use mockall::mock;
    use qrs_datasrc::{DebugTree, TreeInfo};
    use rstest::rstest;

    use crate::{DateTimeBuilder, DateToDateTime};

    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    mock! {
        CutSrc {}

        impl DebugTree for CutSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> TreeInfo;
        }

        impl DataSrc<str> for CutSrc {
            type Output = DateToDateTime;

            fn get(&self, tag: &str) -> anyhow::Result<DateToDateTime>;
        }
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), "tokyo")]
    #[case(ymd(2021, 1, 1), " tokyo@ close")]
    #[case(ymd(2021, 1, 1), "")]
    fn test_display_string(#[case] d: NaiveDate, #[case] tag: &str) {
        let d = DateWithTag {
            date: d,
            tag: tag.to_string(),
        };

        let s = format!("{}", d);

        assert_eq!(s, format!("{}@{}", d.date, d.tag));
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), 42)]
    #[case(ymd(2021, 1, 1), 0)]
    #[case(ymd(2021, 1, 1), -42)]
    fn test_display_i64(#[case] d: NaiveDate, #[case] tag: i64) {
        let d = DateWithTag { date: d, tag };

        let s = format!("{}", d);

        assert_eq!(s, format!("{}@{}", d.date, d.tag));
    }

    #[rstest]
    #[case("2021-01-01@tokyo", Some(DateWithTag { date: ymd(2021, 1, 1), tag: "tokyo".to_string() }))]
    #[case("2021-01-01@ tokyo@ close", Some(DateWithTag { date: ymd(2021, 1, 1), tag: " tokyo@ close".to_string() }))]
    #[case("2021-01-01@", Some(DateWithTag { date: ymd(2021, 1, 1), tag: "".to_string() }))]
    #[case("xx@tokyo", None)]
    #[case("2021-01-01", None)]
    fn test_from_str_string(#[case] s: &str, #[case] exp: Option<DateWithTag>) {
        let res = s.parse::<DateWithTag<String>>();

        if let Some(exp) = exp {
            assert_eq!(res.ok(), Some(exp));
        } else {
            assert!(res.is_err());
        }
    }

    #[rstest]
    #[case("2021-01-01@42", Some(DateWithTag { date: ymd(2021, 1, 1), tag: 42 }))]
    #[case("2021-01-01@42", Some(DateWithTag { date: ymd(2021, 1, 1), tag: 42 }))]
    #[case("xx@42", None)]
    #[case("2021-01-01", None)]
    #[case("2021-01-01@tokyo", None)]
    #[case("2021-01-01@", None)]
    fn test_from_str_i64(#[case] s: &str, #[case] exp: Option<DateWithTag<i64>>) {
        let res = s.parse::<DateWithTag<i64>>();

        if let Some(exp) = exp {
            assert_eq!(res.ok(), Some(exp));
        } else {
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_to_datetime() {
        let mut src = MockCutSrc::new();
        src.expect_get().once().returning(|s| {
            Ok(DateTimeBuilder::default()
                .with_hms(15, 42, 24)
                .with_parsed_timezone(&format!("+{}:00", s)))
        });
        let d = DateWithTag {
            date: ymd(2021, 1, 1),
            tag: "09".to_string(),
        };
        let exp = DateTimeBuilder::default()
            .with_ymd(2021, 1, 1)
            .with_hms(15, 42, 24)
            .with_parsed_timezone("+09:00")
            .build()
            .unwrap();

        let dt = d.to_datetime(&src).unwrap();

        src.checkpoint();
        assert_eq!(dt, exp);
    }

    #[test]
    fn test_to_datetime_propagate_err() {
        let mut src = MockCutSrc::new();
        src.expect_get()
            .once()
            .returning(|_| Err(anyhow!("test error")));
        let d = DateWithTag {
            date: ymd(2021, 1, 1),
            tag: "09".to_string(),
        };

        let res = d.to_datetime(&src);

        assert!(res.is_err());
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(42))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(-42))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(7))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(-7))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(15))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(-15))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(2))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(-2))]
    fn test_add(#[case] d: NaiveDate, #[case] tag: &str, #[case] tenor: Tenor) {
        let d = DateWithTag {
            date: d,
            tag: tag.to_string(),
        };
        let exp = DateWithTag {
            date: d.date + tenor,
            tag: tag.to_string(),
        };

        let d = d + tenor;

        assert_eq!(d, exp);
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(42))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Days(-42))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(7))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Weeks(-7))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(15))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Months(-15))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(2))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(0))]
    #[case(ymd(2021, 1, 1), "tokyo", Tenor::Years(-2))]
    fn test_sub(#[case] d: NaiveDate, #[case] tag: &str, #[case] tenor: Tenor) {
        let d = DateWithTag {
            date: d,
            tag: tag.to_string(),
        };
        let exp = DateWithTag {
            date: d.date - tenor,
            tag: tag.to_string(),
        };

        let d = d - tenor;

        assert_eq!(d, exp);
    }
}
