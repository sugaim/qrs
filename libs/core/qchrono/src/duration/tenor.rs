use std::{
    fmt::Display,
    ops::{Add, Mul, Neg, Sub},
    str::FromStr,
};

use anyhow::bail;
use chrono::{Days, Months, NaiveDate};

// -----------------------------------------------------------------------------
// Tenor
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tenor {
    Days(i16),
    Weeks(i16),
    Months(i16),
    Years(i16),
}

//
//  ser/de
//
impl Display for Tenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (sign, value, suffix) = match self {
            Tenor::Days(d) => (d < &0, d.abs(), "D"),
            Tenor::Weeks(w) => (w < &0, w.abs(), "W"),
            Tenor::Months(m) => (m < &0, m.abs(), "M"),
            Tenor::Years(y) => (y < &0, y.abs(), "Y"),
        };
        write!(f, "{}P{}{}", if sign { "-" } else { "" }, value, suffix)
    }
}

impl FromStr for Tenor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (sign, s) = match s.chars().next() {
            Some('-') => (-1, &s[1..]),
            Some('+') => (1, &s[1..]),
            _ => (1, s),
        };

        let Some(s) = s.strip_prefix('P') else {
            return Err(anyhow::anyhow!(
                "invalid tenor string: {}. Expected format is either of P[n]D, P[n]W, P[n]M, P[n]Y",
                s
            ));
        };
        let n = match s.chars().last() {
            Some(c) if ['D', 'W', 'M', 'Y'].contains(&c) => &s[..s.len() - 1],
            _ => {
                bail!(
                    "invalid tenor string: {s}. Expected format is either of P[n]D, P[n]W, P[n]M, P[n]Y"
                )
            }
        };
        let n = n.parse::<i16>().map_err(|_| {
            anyhow::anyhow!("invalid tenor string: {s}. Fail to parse the number part '{n}'")
        })?;
        match s.chars().last() {
            Some('D') => Ok(Tenor::Days(sign * n)),
            Some('W') => Ok(Tenor::Weeks(sign * n)),
            Some('M') => Ok(Tenor::Months(sign * n)),
            Some('Y') => Ok(Tenor::Years(sign * n)),
            _ => unreachable!(),
        }
    }
}

impl serde::Serialize for Tenor {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Tenor {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Tenor::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Tenor {
    fn schema_name() -> String {
        "Tenor".to_string()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qchrono::Tenor".into()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut obj = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        obj.metadata().description = Some("Tenor string. e.g. P1D, P1W, P1M, P1Y".to_string());
        obj.string().pattern = Some(r#"^[-+]?P\d+[DWMY]$"#.to_string());
        obj.into()
    }
}

//
// ops
//
impl Neg for Tenor {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        match self {
            Tenor::Days(n) => Tenor::Days(-n),
            Tenor::Weeks(n) => Tenor::Weeks(-n),
            Tenor::Months(n) => Tenor::Months(-n),
            Tenor::Years(n) => Tenor::Years(-n),
        }
    }
}

impl Mul<i16> for Tenor {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i16) -> Self::Output {
        match self {
            Tenor::Days(n) => Tenor::Days(n * rhs),
            Tenor::Weeks(n) => Tenor::Weeks(n * rhs),
            Tenor::Months(n) => Tenor::Months(n * rhs),
            Tenor::Years(n) => Tenor::Years(n * rhs),
        }
    }
}

impl Mul<Tenor> for i16 {
    type Output = Tenor;

    #[inline]
    fn mul(self, rhs: Tenor) -> Self::Output {
        rhs * self
    }
}

impl Add<Tenor> for NaiveDate {
    type Output = NaiveDate;

    fn add(self, rhs: Tenor) -> Self::Output {
        match rhs {
            Tenor::Days(d) => {
                if d > 0 {
                    self.checked_add_days(Days::new(d as _))
                        .unwrap_or(NaiveDate::MAX)
                } else {
                    self.checked_sub_days(Days::new(-d as _))
                        .unwrap_or(NaiveDate::MIN)
                }
            }
            Tenor::Weeks(w) => {
                let d = w as i32 * 7;
                if d > 0 {
                    self.checked_add_days(Days::new(d as _))
                        .unwrap_or(NaiveDate::MAX)
                } else {
                    self.checked_sub_days(Days::new(-d as _))
                        .unwrap_or(NaiveDate::MIN)
                }
            }
            Tenor::Months(m) => {
                if m > 0 {
                    self.checked_add_months(Months::new(m as _))
                        .unwrap_or(NaiveDate::MAX)
                } else {
                    self.checked_sub_months(Months::new(-m as _))
                        .unwrap_or(NaiveDate::MIN)
                }
            }
            Tenor::Years(y) => {
                let m = y as i32 * 12;
                if m > 0 {
                    self.checked_add_months(Months::new(m as _))
                        .unwrap_or(NaiveDate::MAX)
                } else {
                    self.checked_sub_months(Months::new(-m as _))
                        .unwrap_or(NaiveDate::MIN)
                }
            }
        }
    }
}

impl Sub<Tenor> for NaiveDate {
    type Output = NaiveDate;

    fn sub(self, rhs: Tenor) -> Self::Output {
        self + -rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_display(
        #[values(0, 1, -1, 42, -42)] n: i16,
        #[values("D", "W", "M", "Y")] suffix: &str,
    ) {
        let tenor = match suffix {
            "D" => Tenor::Days(n),
            "W" => Tenor::Weeks(n),
            "M" => Tenor::Months(n),
            "Y" => Tenor::Years(n),
            _ => unreachable!(),
        };
        let expected = format!("{}P{}{}", if n < 0 { "-" } else { "" }, n.abs(), suffix);

        let tested = tenor.to_string();

        assert_eq!(tested, expected);
    }

    #[rstest]
    fn test_from_str(#[values(0, 1, 42)] n: i16, #[values("D", "W", "M", "Y")] suffix: &str) {
        // non-prefix
        let expected = match suffix {
            "D" => Tenor::Days(n),
            "W" => Tenor::Weeks(n),
            "M" => Tenor::Months(n),
            "Y" => Tenor::Years(n),
            _ => unreachable!(),
        };
        let s = format!("P{}{}", n, suffix);

        let tested = Tenor::from_str(&s).unwrap();

        assert_eq!(tested, expected);

        // positive
        let expected = match suffix {
            "D" => Tenor::Days(n),
            "W" => Tenor::Weeks(n),
            "M" => Tenor::Months(n),
            "Y" => Tenor::Years(n),
            _ => unreachable!(),
        };
        let s = format!("+P{}{}", n.abs(), suffix);

        let tested = Tenor::from_str(&s).unwrap();

        assert_eq!(tested, expected);

        // negative
        let expected = match suffix {
            "D" => Tenor::Days(-n),
            "W" => Tenor::Weeks(-n),
            "M" => Tenor::Months(-n),
            "Y" => Tenor::Years(-n),
            _ => unreachable!(),
        };
        let s = format!("-P{}{}", n.abs(), suffix);

        let tested = Tenor::from_str(&s).unwrap();

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace(" ")]
    #[case::without_prefix("1D")]
    #[case::without_prefix("1W")]
    #[case::without_prefix("1M")]
    #[case::without_prefix("1Y")]
    #[case::invalid_suffix("P1X")]
    #[case::invalid_suffix("P1DW")]
    #[case::invalid_number("P1.0D")]
    #[case::invalid_number("P1.0W")]
    #[case::invalid_number("P1.0M")]
    #[case::invalid_number("P1.0Y")]
    #[case::non_trimmed(" P1D")]
    #[case::non_trimmed("P1D ")]
    #[case::non_trimmed(" P1D ")]
    #[case::non_trimmed("P 1D")]
    #[case::non_trimmed("P1 D")]
    #[case::non_trimmed("P 1 D")]
    fn test_from_str_err(#[case] s: &str) {
        let tested = Tenor::from_str(s);

        assert!(tested.is_err());
    }

    #[rstest]
    fn test_neg(#[values(0, 1, -1, 42, -42)] n: i16) {
        // days
        let tenor = Tenor::Days(n);
        let expected = Tenor::Days(-n);

        let tested = -tenor;

        assert_eq!(tested, expected);

        // weeks
        let tenor = Tenor::Weeks(n);
        let expected = Tenor::Weeks(-n);

        let tested = -tenor;

        assert_eq!(tested, expected);

        // months
        let tenor = Tenor::Months(n);
        let expected = Tenor::Months(-n);

        let tested = -tenor;

        assert_eq!(tested, expected);

        // years
        let tenor = Tenor::Years(n);
        let expected = Tenor::Years(-n);

        let tested = -tenor;

        assert_eq!(tested, expected);
    }

    #[rstest]
    fn test_mul(#[values(0, 1, -1, 42, -42)] n: i16, #[values(0, 1, -1, 42, -42)] m: i16) {
        // days
        let tenor = Tenor::Days(n);
        let expected = Tenor::Days(n * m);

        let tested_l = m * tenor;
        let tested_r = tenor * m;

        assert_eq!(tested_l, expected);
        assert_eq!(tested_r, expected);

        // weeks
        let tenor = Tenor::Weeks(n);
        let expected = Tenor::Weeks(n * m);

        let tested_l = m * tenor;
        let tested_r = tenor * m;

        assert_eq!(tested_l, expected);
        assert_eq!(tested_r, expected);

        // months
        let tenor = Tenor::Months(n);
        let expected = Tenor::Months(n * m);

        let tested_l = m * tenor;
        let tested_r = tenor * m;

        assert_eq!(tested_l, expected);
        assert_eq!(tested_r, expected);

        // years
        let tenor = Tenor::Years(n);
        let expected = Tenor::Years(n * m);

        let tested_l = m * tenor;
        let tested_r = tenor * m;

        assert_eq!(tested_l, expected);
        assert_eq!(tested_r, expected);
    }

    #[rstest]
    #[case((2019, 1, 1), 0, (2019, 1, 1))]
    #[case((2019, 1, 1), 1, (2019, 1, 2))]
    #[case((2019, 1, 1), -1, (2018, 12, 31))]
    #[case((2019, 1, 1), 365, (2020, 1, 1))]
    #[case((2019, 1, 1), -365, (2018, 1, 1))]
    #[case((2020, 1, 1), 365, (2020, 12, 31))]
    #[case((2021, 1, 1), -365, (2020, 1, 2))]

    fn test_add_days(
        #[case] ymd: (i32, u32, u32),
        #[case] tenor: i16,
        #[case] expected: (i32, u32, u32),
    ) {
        let date = NaiveDate::from_ymd_opt(ymd.0, ymd.1, ymd.2).unwrap();
        let expected = NaiveDate::from_ymd_opt(expected.0, expected.1, expected.2).unwrap();

        let tested = date + Tenor::Days(tenor);

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case((2019, 1, 1), 0)]
    #[case((2019, 1, 1), 1)]
    #[case((2019, 1, 1), -1)]
    #[case((2019, 1, 1), 53)]
    #[case((2019, 1, 1), -53)]
    #[case((2020, 1, 1), 53)]
    #[case((2021, 1, 1), -53)]
    fn test_add_weeks(#[case] base: (i32, u32, u32), #[case] tenor: i16) {
        let date = NaiveDate::from_ymd_opt(base.0, base.1, base.2).unwrap();
        let expected = date + Tenor::Days(tenor * 7);

        let tested = date + Tenor::Weeks(tenor);

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case::typical((2019, 1, 1), 0, (2019, 1, 1))]
    #[case::typical((2019, 1, 1), 1, (2019, 2, 1))]
    #[case::typical((2019, 1, 1), -1, (2018, 12, 1))]
    #[case::typical((2019, 1, 1), 12, (2020, 1, 1))]
    #[case::typical((2019, 1, 1), -12, (2018, 1, 1))]
    #[case::not_exist((2019, 1, 31), 1, (2019, 2, 28))]
    #[case::not_exist((2019, 3, 31), -1, (2019, 2, 28))]
    #[case::not_exist((2020, 1, 31), 1, (2020, 2, 29))]
    #[case::not_exist((2020, 2, 29), 12, (2021, 2, 28))]
    #[case::not_exist((2020, 2, 29), -12, (2019, 2, 28))]
    fn test_add_months(
        #[case] base: (i32, u32, u32),
        #[case] tenor: i16,
        #[case] expected: (i32, u32, u32),
    ) {
        let date = NaiveDate::from_ymd_opt(base.0, base.1, base.2).unwrap();
        let expected = NaiveDate::from_ymd_opt(expected.0, expected.1, expected.2).unwrap();

        let tested = date + Tenor::Months(tenor);

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case::typical((2019, 1, 1), 0)]
    #[case::typical((2019, 1, 1), 1)]
    #[case::typical((2019, 1, 1), -1)]
    #[case::typical((2019, 1, 1), 10)]
    #[case::typical((2019, 1, 1), -10)]
    #[case::not_exist((2020, 2, 29), 1)]
    #[case::not_exist((2020, 2, 29), -1)]
    fn test_add_years(#[case] base: (i32, u32, u32), #[case] tenor: i16) {
        let date = NaiveDate::from_ymd_opt(base.0, base.1, base.2).unwrap();
        let expected = date + Tenor::Months(tenor * 12);

        let tested = date + Tenor::Years(tenor);

        assert_eq!(tested, expected);
    }

    #[rstest]
    #[case((2019, 1, 1), Tenor::Days(0))]
    #[case((2019, 1, 1), Tenor::Days(1))]
    #[case((2019, 1, 1), Tenor::Days(-1))]
    #[case((2019, 1, 1), Tenor::Days(365))]
    #[case((2019, 1, 1), Tenor::Days(-365))]
    #[case((2020, 1, 1), Tenor::Days(-365))]
    #[case((2021, 1, 1), Tenor::Days(365))]
    #[case((2019, 1, 1), Tenor::Weeks(0))]
    #[case((2019, 1, 1), Tenor::Weeks(1))]
    #[case((2019, 1, 1), Tenor::Weeks(-1))]
    #[case((2019, 1, 1), Tenor::Weeks(-53))]
    #[case((2019, 1, 1), Tenor::Weeks(53))]
    #[case((2020, 1, 1), Tenor::Weeks(-53))]
    #[case((2021, 1, 1), Tenor::Weeks(53))]
    #[case((2019, 1, 1), Tenor::Months(0))]
    #[case((2019, 1, 1), Tenor::Months(1))]
    #[case((2019, 1, 1), Tenor::Months(-1))]
    #[case((2019, 1, 1), Tenor::Months(-12))]
    #[case((2019, 1, 1), Tenor::Months(12))]
    #[case((2019, 1, 31), Tenor::Months(1))]
    #[case((2019, 3, 31), Tenor::Months(-1))]
    #[case((2020, 1, 31), Tenor::Months(1))]
    #[case((2020, 2, 29), Tenor::Months(-12))]
    #[case((2020, 2, 29), Tenor::Months(12))]
    #[case((2020, 2, 29), Tenor::Months(-1))]
    #[case((2020, 2, 29), Tenor::Months(1))]
    #[case((2019, 1, 1), Tenor::Years(0))]
    #[case((2019, 1, 1), Tenor::Years(1))]
    #[case((2019, 1, 1), Tenor::Years(-1))]
    #[case((2019, 1, 1), Tenor::Years(10))]
    #[case((2019, 1, 1), Tenor::Years(-10))]
    #[case((2020, 2, 29), Tenor::Years(1))]
    #[case((2020, 2, 29), Tenor::Years(1))]
    fn test_sub(#[case] base: (i32, u32, u32), #[case] tenor: Tenor) {
        let date = NaiveDate::from_ymd_opt(base.0, base.1, base.2).unwrap();
        let expected = date + -tenor;

        let tested = date - tenor;

        assert_eq!(tested, expected);
    }
}
