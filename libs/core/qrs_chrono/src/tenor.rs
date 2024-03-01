use std::{fmt::Display, str::FromStr};

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
// display, serde
//
#[cfg(feature = "serde")]
impl serde::Serialize for Tenor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tenor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl schemars::JsonSchema for Tenor {
    fn schema_name() -> String {
        "Tenor".to_string()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_chrono::Tenor".into()
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        res.string().pattern = Some(r#"^[-+]?P[-+]?\d+(D|W|M|Y)$"#.to_string());
        res.metadata().description = Some("String representing date shift".to_string());
        res.metadata().examples = vec![
            serde_json::json!("P1D"),
            serde_json::json!("-P1D"),
            serde_json::json!("P1W"),
            serde_json::json!("-P1W"),
            serde_json::json!("P1M"),
            serde_json::json!("-P1M"),
            serde_json::json!("P1Y"),
            serde_json::json!("-P1Y"),
        ];
        res.into()
    }
}

//
// construction
//
impl Display for Tenor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tenor::Days(d) => {
                if d < &0 {
                    write!(f, "-P{}D", -d)
                } else {
                    write!(f, "P{}D", d)
                }
            }
            Tenor::Weeks(w) => {
                if w < &0 {
                    write!(f, "-P{}W", -w)
                } else {
                    write!(f, "P{}W", w)
                }
            }
            Tenor::Months(m) => {
                if m < &0 {
                    write!(f, "-P{}M", -m)
                } else {
                    write!(f, "P{}M", m)
                }
            }
            Tenor::Years(y) => {
                if y < &0 {
                    write!(f, "-P{}Y", -y)
                } else {
                    write!(f, "P{}Y", y)
                }
            }
        }
    }
}

impl FromStr for Tenor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (sign, s) = if let Some(s) = s.strip_prefix('-') {
            (-1, s)
        } else if let Some(s) = s.strip_prefix('+') {
            (1, s)
        } else {
            (1, s)
        };

        // either or
        // P[n]D, P[n]W, P[n]M, P[n]Y
        let Some(s) = s.strip_prefix('P') else {
            return Err(anyhow::anyhow!(
                "invalid tenor string: {}. Expected format is either of P[n]D, P[n]W, P[n]M, P[n]Y",
                s
            ));
        };
        if let Some(s) = s.strip_suffix('D') {
            s.parse().map(Tenor::Days).map(|n| sign * n).map_err(|_| {
                anyhow::anyhow!(
                    "Fail to parse integer part 'n' of P[n]D. Note that mixed tenor is not supported."
                )
            })
        } else if let Some(s) = s.strip_suffix('W') {
            s.parse().map(Tenor::Weeks).map(|n| sign * n).map_err(|_| {
                anyhow::anyhow!(
                    "Fail to parse integer part 'n' of P[n]W. Note that mixed tenor is not supported."
                )
            })
        } else if let Some(s) = s.strip_suffix('M') {
            s.parse().map(Tenor::Months).map(|n| sign * n).map_err(|_| {
                anyhow::anyhow!(
                    "Fail to parse integer part 'n' of P[n]M. Note that mixed tenor is not supported."
                )
            })
        } else if let Some(s) = s.strip_suffix('Y') {
            s.parse().map(Tenor::Years).map(|n| sign * n).map_err(|_| {
                anyhow::anyhow!(
                    "Fail to parse integer part 'n' of P[n]Y. Note that mixed tenor is not supported."
                )
            })
        } else {
            Err(anyhow::anyhow!(
                "invalid tenor string: {}. Expected format is either of P[n]D, P[n]W, P[n]M, P[n]Y",
                s
            ))
        }
    }
}

//
// operators
//
impl std::ops::Neg for Tenor {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        match self {
            Tenor::Days(d) => Tenor::Days(-d),
            Tenor::Weeks(w) => Tenor::Weeks(-w),
            Tenor::Months(m) => Tenor::Months(-m),
            Tenor::Years(y) => Tenor::Years(-y),
        }
    }
}

impl std::ops::Mul<i16> for Tenor {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i16) -> Self::Output {
        match self {
            Tenor::Days(d) => Tenor::Days(d * rhs),
            Tenor::Weeks(w) => Tenor::Weeks(w * rhs),
            Tenor::Months(m) => Tenor::Months(m * rhs),
            Tenor::Years(y) => Tenor::Years(y * rhs),
        }
    }
}

impl std::ops::Mul<Tenor> for i16 {
    type Output = Tenor;

    #[inline]
    fn mul(self, rhs: Tenor) -> Self::Output {
        rhs * self
    }
}

impl std::ops::Add<Tenor> for NaiveDate {
    type Output = NaiveDate;

    #[inline]
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

impl std::ops::Sub<Tenor> for NaiveDate {
    type Output = NaiveDate;

    #[inline]
    fn sub(self, rhs: Tenor) -> Self::Output {
        self + -rhs
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(Tenor::Days(1).to_string(), "P1D");
        assert_eq!(Tenor::Days(-1).to_string(), "-P1D");
        assert_eq!(Tenor::Weeks(1).to_string(), "P1W");
        assert_eq!(Tenor::Weeks(-1).to_string(), "-P1W");
        assert_eq!(Tenor::Months(1).to_string(), "P1M");
        assert_eq!(Tenor::Months(-1).to_string(), "-P1M");
        assert_eq!(Tenor::Years(1).to_string(), "P1Y");
        assert_eq!(Tenor::Years(-1).to_string(), "-P1Y");
    }

    #[test]
    fn test_from_str() {
        let signs = ["", "-", "+"];
        let units = ["D", "W", "M", "Y"];
        for overall_sign in signs {
            for sign in signs {
                for unit in units {
                    for n in 0..10 {
                        let s = format!("{}P{}{}{}", overall_sign, sign, n, unit);
                        let expected_sign = if (sign == "-") ^ (overall_sign == "-") {
                            -1
                        } else {
                            1
                        };
                        let expected = match unit {
                            "D" => Tenor::Days(expected_sign * n as i16),
                            "W" => Tenor::Weeks(expected_sign * n as i16),
                            "M" => Tenor::Months(expected_sign * n as i16),
                            "Y" => Tenor::Years(expected_sign * n as i16),
                            _ => unreachable!(),
                        };
                        assert_eq!(s.parse::<Tenor>().unwrap(), expected);
                    }
                }
            }
        }

        // error: spaces
        assert!(" P1D".parse::<Tenor>().is_err());
        assert!("- P1D".parse::<Tenor>().is_err());
        assert!("-P1D ".parse::<Tenor>().is_err());
        assert!("P 1D".parse::<Tenor>().is_err());
        assert!("-P 1D".parse::<Tenor>().is_err());
        assert!("P 1W".parse::<Tenor>().is_err());
        assert!("-P 1W".parse::<Tenor>().is_err());
        assert!("P 1M".parse::<Tenor>().is_err());
        assert!("-P 1M".parse::<Tenor>().is_err());
        assert!("P 1Y".parse::<Tenor>().is_err());
        assert!("-P 1Y".parse::<Tenor>().is_err());
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Tenor::Days(1), Tenor::Days(-1));
        assert_eq!(-Tenor::Weeks(1), Tenor::Weeks(-1));
        assert_eq!(-Tenor::Months(1), Tenor::Months(-1));
        assert_eq!(-Tenor::Years(1), Tenor::Years(-1));
    }

    #[test]
    fn test_mul() {
        let tenors_counts = [0, 1, 2, 3, 4, 5, -1, -2, -3, -4, -5];
        let scals = [0, 1, 2, 3, 4, 5, -1, -2, -3, -4, -5];
        for tenor_count in tenors_counts {
            for scal in scals {
                assert_eq!(
                    Tenor::Days(tenor_count) * scal,
                    Tenor::Days(tenor_count * scal)
                );
                assert_eq!(
                    scal * Tenor::Days(tenor_count),
                    Tenor::Days(tenor_count * scal)
                );
                assert_eq!(
                    Tenor::Weeks(tenor_count) * scal,
                    Tenor::Weeks(tenor_count * scal)
                );
                assert_eq!(
                    scal * Tenor::Weeks(tenor_count),
                    Tenor::Weeks(tenor_count * scal)
                );
                assert_eq!(
                    Tenor::Months(tenor_count) * scal,
                    Tenor::Months(tenor_count * scal)
                );
                assert_eq!(
                    scal * Tenor::Months(tenor_count),
                    Tenor::Months(tenor_count * scal)
                );
                assert_eq!(
                    Tenor::Years(tenor_count) * scal,
                    Tenor::Years(tenor_count * scal)
                );
                assert_eq!(
                    scal * Tenor::Years(tenor_count),
                    Tenor::Years(tenor_count * scal)
                );
            }
        }
    }

    #[test]
    fn test_add() {
        let ymd = |y: i32, m: u32, d: u32| NaiveDate::from_ymd_opt(y, m, d).unwrap();

        // days
        assert_eq!(ymd(2021, 1, 1) + Tenor::Days(1), ymd(2021, 1, 2));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Days(-1), ymd(2020, 12, 31));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Days(0), ymd(2021, 1, 1));

        // month change
        assert_eq!(ymd(2021, 1, 31) + Tenor::Days(1), ymd(2021, 2, 1));
        assert_eq!(ymd(2021, 2, 1) + Tenor::Days(-1), ymd(2021, 1, 31));

        // leap, non-leap
        assert_eq!(ymd(2020, 2, 28) + Tenor::Days(1), ymd(2020, 2, 29));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Days(1), ymd(2020, 3, 1));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Days(-1), ymd(2020, 2, 28));
        assert_eq!(ymd(2020, 3, 1) + Tenor::Days(-1), ymd(2020, 2, 29));

        assert_eq!(ymd(2021, 2, 28) + Tenor::Days(1), ymd(2021, 3, 1));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Days(2), ymd(2021, 3, 2));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Days(-1), ymd(2021, 2, 27));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Days(-2), ymd(2021, 2, 26));

        // weeks
        assert_eq!(ymd(2021, 1, 1) + Tenor::Weeks(1), ymd(2021, 1, 8));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Weeks(-1), ymd(2020, 12, 25));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Weeks(0), ymd(2021, 1, 1));

        // month change
        assert_eq!(ymd(2021, 1, 31) + Tenor::Weeks(1), ymd(2021, 2, 7));
        assert_eq!(ymd(2021, 2, 7) + Tenor::Weeks(-1), ymd(2021, 1, 31));

        // leap, non-leap
        assert_eq!(ymd(2020, 2, 28) + Tenor::Weeks(1), ymd(2020, 3, 6));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Weeks(1), ymd(2020, 3, 7));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Weeks(-1), ymd(2020, 2, 22));
        assert_eq!(ymd(2020, 3, 7) + Tenor::Weeks(-1), ymd(2020, 2, 29));

        assert_eq!(ymd(2021, 2, 28) + Tenor::Weeks(1), ymd(2021, 3, 7));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Weeks(2), ymd(2021, 3, 14));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Weeks(-1), ymd(2021, 2, 21));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Weeks(-2), ymd(2021, 2, 14));

        // months
        assert_eq!(ymd(2021, 1, 1) + Tenor::Months(1), ymd(2021, 2, 1));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Months(-1), ymd(2020, 12, 1));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Months(0), ymd(2021, 1, 1));

        // when not exist
        assert_eq!(ymd(2021, 1, 31) + Tenor::Months(1), ymd(2021, 2, 28));
        assert_eq!(ymd(2021, 1, 31) + Tenor::Months(-2), ymd(2020, 11, 30));

        // leap, non-leap
        assert_eq!(ymd(2020, 2, 29) + Tenor::Months(1), ymd(2020, 3, 29));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Months(-1), ymd(2020, 1, 29));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Months(2), ymd(2020, 4, 29));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Months(-2), ymd(2019, 12, 29));

        assert_eq!(ymd(2021, 2, 28) + Tenor::Months(1), ymd(2021, 3, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Months(-1), ymd(2021, 1, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Months(2), ymd(2021, 4, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Months(-2), ymd(2020, 12, 28));

        // years
        assert_eq!(ymd(2021, 1, 1) + Tenor::Years(1), ymd(2022, 1, 1));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Years(-1), ymd(2020, 1, 1));
        assert_eq!(ymd(2021, 1, 1) + Tenor::Years(0), ymd(2021, 1, 1));

        // leap, non-leap
        assert_eq!(ymd(2020, 2, 29) + Tenor::Years(1), ymd(2021, 2, 28));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Years(-1), ymd(2019, 2, 28));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Years(2), ymd(2022, 2, 28));
        assert_eq!(ymd(2020, 2, 29) + Tenor::Years(-2), ymd(2018, 2, 28));

        assert_eq!(ymd(2021, 2, 28) + Tenor::Years(1), ymd(2022, 2, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Years(-1), ymd(2020, 2, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Years(2), ymd(2023, 2, 28));
        assert_eq!(ymd(2021, 2, 28) + Tenor::Years(-2), ymd(2019, 2, 28));
    }

    #[test]
    fn test_sub() {
        let ymd = |y: i32, m: u32, d: u32| NaiveDate::from_ymd_opt(y, m, d).unwrap();

        for y in 2020..2023 {
            for m in 1..=12 {
                for d in 1..=28 {
                    let date = ymd(y, m, d);
                    for tenor in [
                        Tenor::Days(0),
                        Tenor::Weeks(0),
                        Tenor::Months(0),
                        Tenor::Years(0),
                        Tenor::Days(1),
                        Tenor::Weeks(1),
                        Tenor::Months(1),
                        Tenor::Years(1),
                    ] {
                        assert_eq!(date - tenor, date + -tenor);
                        assert_eq!(date - (-tenor), date + tenor);
                    }
                }
            }
        }
    }
}
