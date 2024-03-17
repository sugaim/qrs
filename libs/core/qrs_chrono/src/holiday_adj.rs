use anyhow::anyhow;
use chrono::Days;

use crate::{Calendar, Datelike, NaiveDate};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

// -----------------------------------------------------------------------------
// HolidayAdj
//
/// Adjustment rule for holiday.
///
/// # Overview
/// Adjustment rule for holiday, which is typically used after
/// date calculation such as adding 1 month to the date.
///
/// Since the meaning of "holiday" depends on the calendar,
/// this adjustment rule is used with the calendar.
///
/// # Examples
/// ```
/// use qrs_chrono::{Calendar, HolidayAdj};
/// use chrono::NaiveDate as Date;
///
/// let cal = Calendar::default();
/// let d = Date::from_ymd_opt(2023, 12, 31).unwrap();
///
/// // Following: Subday is shifted to the next business day.
/// let rule = HolidayAdj::Following;
/// assert_eq!(Date::from_ymd_opt(2024, 1, 1).unwrap(), rule.adjust(d, &cal).unwrap());
///
/// // Modified following: Shifted with following rule reaches the next month and shifted bask
/// let rule = HolidayAdj::ModifiedFollowing;
/// assert_eq!(Date::from_ymd_opt(2023, 12, 29).unwrap(), rule.adjust(d, &cal).unwrap());
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HolidayAdj {
    /// No adjustment
    #[strum(serialize = "unadjust")]
    Unadjust,

    /// Shift to the next business day for the holiday.
    #[strum(serialize = "following")]
    Following,

    /// Shift to the next business day for the holiday,
    /// but if it reaches the next month, shift back to the previous business day.
    #[strum(serialize = "modified_following")]
    ModifiedFollowing,

    /// Shift to the previous business day for the holiday.
    #[strum(serialize = "preceding")]
    Preceding,

    /// Shift to the previous business day for the holiday,
    /// but if it reaches the previous month, shift forward to the next business day.
    #[strum(serialize = "modified_preceding")]
    ModifiedPreceding,
}

//
// methods
//
impl HolidayAdj {
    /// Apply the adjustment rule to the date.
    ///
    /// The calendar has valid period.
    /// If the date, including shifted date, reaches the out of valid period,
    /// this function returns [`None`].
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::{Calendar, HolidayAdj};
    /// use chrono::NaiveDate as Date;
    ///
    /// let cal = Calendar::default();
    /// let d = Date::from_ymd_opt(2023, 12, 31).unwrap();
    ///
    /// // Following: Subday is shifted to the next business day.
    /// assert_eq!(Date::from_ymd_opt(2024, 1, 1).unwrap(), HolidayAdj::Following.adjust(d, &cal).unwrap());
    ///
    /// // Modified following: Shifted with following rule reaches the next month and shifted bask
    /// assert_eq!(Date::from_ymd_opt(2023, 12, 29).unwrap(), HolidayAdj::ModifiedFollowing.adjust(d, &cal).unwrap());
    /// ```
    pub fn adjust(&self, d: NaiveDate, cal: &Calendar) -> anyhow::Result<NaiveDate> {
        match self {
            HolidayAdj::Unadjust => cal.validate(d).map_err(Into::into),
            HolidayAdj::Following => {
                let mut d = d;
                while cal.is_holiday(d).map_err(Into::<anyhow::Error>::into)? {
                    d = d
                        .checked_add_days(Days::new(1))
                        .ok_or_else(|| anyhow!("date is overflow"))?;
                }
                Ok(d)
            }
            HolidayAdj::Preceding => {
                let mut d = d;
                while cal.is_holiday(d).map_err(Into::<anyhow::Error>::into)? {
                    d = d
                        .checked_sub_days(Days::new(1))
                        .ok_or_else(|| anyhow!("date is underflow"))?;
                }
                Ok(d)
            }
            HolidayAdj::ModifiedFollowing => {
                let adjusted = Self::Following.adjust(d, cal)?;
                if adjusted.month() != d.month() {
                    d.checked_sub_days(Days::new(1))
                        .ok_or_else(|| anyhow!("date is underflow"))
                        .and_then(|d| Self::Preceding.adjust(d, cal))
                } else {
                    Ok(adjusted)
                }
            }
            HolidayAdj::ModifiedPreceding => {
                let adjusted = Self::Preceding.adjust(d, cal)?;
                if adjusted.month() != d.month() {
                    d.checked_add_days(Days::new(1))
                        .ok_or_else(|| anyhow!("date is overflow"))
                        .and_then(|d| Self::Following.adjust(d, cal))
                } else {
                    Ok(adjusted)
                }
            }
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test_display() {
        assert_eq!("unadjust", HolidayAdj::Unadjust.to_string());
        assert_eq!("following", HolidayAdj::Following.to_string());
        assert_eq!(
            "modified_following",
            HolidayAdj::ModifiedFollowing.to_string()
        );
        assert_eq!("preceding", HolidayAdj::Preceding.to_string());
        assert_eq!(
            "modified_preceding",
            HolidayAdj::ModifiedPreceding.to_string()
        );
    }

    #[test]
    fn test_from_str() {
        for e in HolidayAdj::iter() {
            assert_eq!(Ok(e), e.to_string().parse());
        }
    }

    #[test]
    fn test_adj() {
        let ymd = |y: i32, m: u32, d: u32| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let cal = Calendar::builder()
            .with_valid_period(ymd(2023, 12, 28), ymd(2024, 1, 10))
            .with_extra_business_days(vec![])
            .with_extra_holidays(vec![ymd(2024, 1, 1), ymd(2024, 1, 8)])
            .build()
            .unwrap();

        // 2023-12-28 ~ 2024-01-12
        let days: Vec<_> = ymd(2023, 12, 26).iter_days().take(17).collect();

        use HolidayAdj::*;
        // unadjust
        for day in days.iter() {
            if day < &ymd(2023, 12, 28) || &ymd(2024, 1, 10) <= day {
                assert!(Unadjust.adjust(*day, &cal).is_err());
            } else {
                assert_eq!(day, &Unadjust.adjust(*day, &cal).unwrap());
            }
        }

        // following
        let exp = hashmap! {
            ymd(2023, 12, 26) => None,
            ymd(2023, 12, 27) => None,
            ymd(2023, 12, 30) => Some(ymd(2024, 1, 2)),
            ymd(2023, 12, 31) => Some(ymd(2024, 1, 2)),
            ymd(2024, 1, 1) => Some(ymd(2024, 1, 2)),
            ymd(2024, 1, 6) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 7) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 8) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 10) => None,
            ymd(2024, 1, 11) => None,
        };
        for day in days.iter() {
            let tested = Following.adjust(*day, &cal);
            if let Some(d) = exp.get(day) {
                match d {
                    Some(d) => assert_eq!(*d, tested.unwrap()),
                    None => assert!(tested.is_err()),
                }
            } else {
                assert_eq!(*day, tested.unwrap());
            }
        }

        // modified_following
        let exp = hashmap! {
            ymd(2023, 12, 26) => None,
            ymd(2023, 12, 27) => None,
            ymd(2023, 12, 30) => Some(ymd(2023, 12, 29)),
            ymd(2023, 12, 31) => Some(ymd(2023, 12, 29)),
            ymd(2024, 1, 1) => Some(ymd(2024, 1, 2)),
            ymd(2024, 1, 6) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 7) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 8) => Some(ymd(2024, 1, 9)),
            ymd(2024, 1, 10) => None,
            ymd(2024, 1, 11) => None,
        };
        for day in days.iter() {
            let tested = ModifiedFollowing.adjust(*day, &cal);
            if let Some(d) = exp.get(day) {
                match d {
                    Some(d) => assert_eq!(*d, tested.unwrap()),
                    None => assert!(tested.is_err()),
                }
            } else {
                assert_eq!(*day, tested.unwrap());
            }
        }

        // preceding
        let exp = hashmap! {
            ymd(2023, 12, 26) => None,
            ymd(2023, 12, 27) => None,
            ymd(2023, 12, 30) => Some(ymd(2023, 12, 29)),
            ymd(2023, 12, 31) => Some(ymd(2023, 12, 29)),
            ymd(2024, 1, 1) => Some(ymd(2023, 12, 29)),
            ymd(2024, 1, 6) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 7) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 8) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 10) => None,
            ymd(2024, 1, 11) => None,
        };
        for day in days.iter() {
            let tested = Preceding.adjust(*day, &cal);
            if let Some(d) = exp.get(day) {
                match d {
                    Some(d) => assert_eq!(*d, tested.unwrap()),
                    None => assert!(tested.is_err()),
                }
            } else {
                assert_eq!(*day, tested.unwrap());
            }
        }

        // modified_preceding
        let exp = hashmap! {
            ymd(2023, 12, 26) => None,
            ymd(2023, 12, 27) => None,
            ymd(2023, 12, 30) => Some(ymd(2023, 12, 29)),
            ymd(2023, 12, 31) => Some(ymd(2023, 12, 29)),
            ymd(2024, 1, 1) => Some(ymd(2024, 1, 2)),
            ymd(2024, 1, 6) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 7) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 8) => Some(ymd(2024, 1, 5)),
            ymd(2024, 1, 10) => None,
            ymd(2024, 1, 11) => None,
        };
        for day in days.iter() {
            let tested = ModifiedPreceding.adjust(*day, &cal);
            if let Some(d) = exp.get(day) {
                match d {
                    Some(d) => assert_eq!(*d, tested.unwrap()),
                    None => assert!(tested.is_err()),
                }
            } else {
                assert_eq!(*day, tested.unwrap());
            }
        }
    }
}
