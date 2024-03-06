use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;
use chrono::NaiveDate;
use qrs_datasrc::DataSrc;

use crate::{DateTime, Tenor, TimeCut};

// -----------------------------------------------------------------------------
// DateWithTimeCutSym
//
/// Date with a time cut symbol.
///
/// Actual values of time cut can be varying.
/// For example, if 'tokyo close' means close time at tokyo trading market,
/// the time cut can be changed when the market open time is changed.
///
/// In such cases, introducing a symbol to represent the time cut is useful
/// for dayly operations.
///
/// # String representation
/// The string representation of this type is `{{date}}@{{sym}}`.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateWithTimeCut<Sym> {
    pub date: NaiveDate,
    pub sym: Sym,
}

//
// display, serde
//
impl<Sym: Display> Display for DateWithTimeCut<Sym> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.date, self.sym)
    }
}

impl<Syn: FromStr> FromStr for DateWithTimeCut<Syn>
where
    anyhow::Error: From<<Syn as FromStr>::Err>,
{
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (date, sym) = s.split_once('@').ok_or_else(|| {
            anyhow!(
                "Fail to parse DateWithTimeCut from string: {}. The string must contain '@' to separate date and symbol",
                s
            )
        })?;
        Ok(DateWithTimeCut {
            date: date
                .parse()
                .map_err(|_| anyhow!("Fail to parse date from string: {}", date))?,
            sym: sym.parse()?,
        })
    }
}

#[cfg(feature = "serde")]
impl<Sym> serde::Serialize for DateWithTimeCut<Sym>
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
impl<'de, Sym> serde::Deserialize<'de> for DateWithTimeCut<Sym>
where
    Sym: FromStr,
    anyhow::Error: From<<Sym as FromStr>::Err>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<DateWithTimeCut<Sym>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateWithTimeCut::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<Sym> schemars::JsonSchema for DateWithTimeCut<Sym>
where
    Sym: schemars::JsonSchema + Display + FromStr,
{
    fn schema_name() -> String {
        format!("DateWithTimeCut_for_{}", Sym::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_chrono::DateWithTimeCut<{}>", Sym::schema_id()).into()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            format: Some("date-time".to_owned()),
            ..Default::default()
        };
        res.metadata().description =
            Some("Date with a time cut symbol. Format is 'yyyy-MM-dd@{sym}'".to_owned());
        res.into()
    }
}

//
// methods
//
impl<Sym> DateWithTimeCut<Sym> {
    #[inline]
    pub fn to_datetime<Tz, Cut, Res>(&self, resolver: &Res) -> Result<DateTime<Tz>, anyhow::Error>
    where
        Tz: chrono::TimeZone,
        Cut: TimeCut<Tz = Tz>,
        Res: DataSrc<Sym, Output = Cut>,
        anyhow::Error: From<Cut::Err>,
    {
        let cut = resolver.get(&self.sym)?;
        cut.to_datetime(self.date).map_err(Into::into)
    }
}

//
// operators
//
impl<Sym> std::ops::Add<Tenor> for DateWithTimeCut<Sym> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Tenor) -> Self::Output {
        DateWithTimeCut {
            date: self.date + rhs,
            sym: self.sym,
        }
    }
}

impl<Sym> std::ops::Sub<Tenor> for DateWithTimeCut<Sym> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Tenor) -> Self::Output {
        DateWithTimeCut {
            date: self.date - rhs,
            sym: self.sym,
        }
    }
}

// // =============================================================================
// #[cfg(test)]
// mod tests {
//     use maplit::hashmap;
//     use qrs_datasrc::InMemory;
//     use rstest::{fixture, rstest};

//     use crate::{DateTimeBuilder, DateToDateTime};

//     use super::*;

//     #[fixture]
//     fn datasrc() -> InMemory<String, DateToDateTime> {
//         let data = hashmap! {
//             "tky-close".to_owned() => DateTimeBuilder::new()
//                 .with_hms(15, 30, 0)
//                 .with_parsed_timezone("+09:00"),
//             "nyk-close".to_owned() => DateTimeBuilder::new()
//                 .with_hms(15, 30, 0)
//                 .with_parsed_timezone("America/New_York"),
//         };
//         data.into()
//     }

//     fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
//         NaiveDate::from_ymd_opt(y, m, d).unwrap()
//     }

//     #[test]
//     fn test_display() {
//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: "tky-close",
//         };
//         assert_eq!(format!("{}", dwtc), "2021-01-01@tky-close");

//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: "nyk-close",
//         };
//         assert_eq!(format!("{}", dwtc), "2021-01-01@nyk-close");

//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: 1,
//         };
//         assert_eq!(format!("{}", dwtc), "2021-01-01@1");
//     }

//     #[test]
//     fn test_from_str() {
//         let s = "2021-01-01@tky-close";
//         let dwtc: DateWithTimeCut<String> = s.parse().unwrap();
//         assert_eq!(dwtc.date, ymd(2021, 1, 1));
//         assert_eq!(dwtc.sym, "tky-close");

//         let s = "2021-01-01@nyk-close";
//         let dwtc: DateWithTimeCut<String> = s.parse().unwrap();
//         assert_eq!(dwtc.date, ymd(2021, 1, 1));
//         assert_eq!(dwtc.sym, "nyk-close");

//         let s = "2021-01-01@1";
//         let dwtc = s.parse::<DateWithTimeCut<i32>>().unwrap();
//         assert_eq!(dwtc.date, ymd(2021, 1, 1));
//         assert_eq!(dwtc.sym, 1);

//         // errors
//         let s = "2021-01-01";
//         assert!(s.parse::<DateWithTimeCut<String>>().is_err());

//         let s = "2021@tky-close";
//         assert!(s.parse::<DateWithTimeCut<String>>().is_err());
//     }

//     #[rstest]
//     fn test_to_datetime(datasrc: InMemory<String, DateToDateTime>) {
//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: "tky-close".to_owned(),
//         };
//         let dt = dwtc.to_datetime(&datasrc).unwrap();
//         assert_eq!(
//             dt,
//             datasrc
//                 .get("tky-close")
//                 .unwrap()
//                 .data
//                 .to_datetime(ymd(2021, 1, 1))
//                 .unwrap()
//         );

//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: "nyk-close".to_owned(),
//         };
//         let dt = dwtc.to_datetime(&datasrc).unwrap();
//         assert_eq!(
//             dt,
//             datasrc
//                 .get("nyk-close")
//                 .unwrap()
//                 .data
//                 .to_datetime(ymd(2021, 1, 1))
//                 .unwrap()
//         );

//         // errors
//         let dwtc = DateWithTimeCut {
//             date: ymd(2021, 1, 1),
//             sym: "unknown".to_owned(),
//         };
//         assert!(dwtc.to_datetime(&datasrc).is_err());
//     }

//     #[test]
//     fn test_add_sub() {
//         let bases = vec![
//             DateWithTimeCut {
//                 date: ymd(2021, 1, 1),
//                 sym: "tky-close",
//             },
//             DateWithTimeCut {
//                 date: ymd(2021, 1, 1),
//                 sym: "nyk-close",
//             },
//             DateWithTimeCut {
//                 date: ymd(2020, 2, 29),
//                 sym: "tky-close",
//             },
//             DateWithTimeCut {
//                 date: ymd(2021, 2, 28),
//                 sym: "nyk-close",
//             },
//         ];
//         let tenors = vec![
//             Tenor::Days(0),
//             Tenor::Days(1),
//             Tenor::Days(2),
//             Tenor::Weeks(0),
//             Tenor::Weeks(1),
//             Tenor::Weeks(2),
//             Tenor::Months(0),
//             Tenor::Months(1),
//             Tenor::Months(2),
//             Tenor::Years(0),
//             Tenor::Years(1),
//             Tenor::Years(2),
//         ];
//         for base in &bases {
//             for tenor in &tenors {
//                 let dwtc = *base + *tenor;
//                 assert_eq!(dwtc.date, base.date + *tenor);
//                 assert_eq!(dwtc.sym, base.sym);

//                 let dwtc = *base - *tenor;
//                 assert_eq!(dwtc.date, base.date - *tenor);
//                 assert_eq!(dwtc.sym, base.sym);
//             }
//         }
//     }
// }
