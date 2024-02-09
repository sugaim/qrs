use std::{
    collections::HashSet,
    ops::{BitAnd, BitOr},
    sync::Arc,
};

use anyhow::ensure;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize};

// -----------------------------------------------------------------------------
// CalendarData
//
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
struct CalendarData {
    #[serde(rename = "extra_holidays")]
    extra_holds: Vec<NaiveDate>,
    #[serde(rename = "extra_business_days")]
    extra_bizds: Vec<NaiveDate>,

    valid_from: NaiveDate, // inclusive
    valid_to: NaiveDate,   // exclusive
}

//
// display, serde
//
impl<'de> Deserialize<'de> for CalendarData {
    fn deserialize<D>(deserializer: D) -> Result<CalendarData, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct _Data {
            extra_holidays: Vec<NaiveDate>,
            extra_business_days: Vec<NaiveDate>,
            valid_from: NaiveDate,
            valid_to: NaiveDate,
        }

        let data = _Data::deserialize(deserializer)?;
        CalendarData::new(
            data.extra_holidays,
            data.extra_business_days,
            data.valid_from,
            data.valid_to,
        )
        .map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl CalendarData {
    fn new(
        mut extra_holds: Vec<NaiveDate>,
        mut extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
    ) -> anyhow::Result<Self> {
        ensure!(
            valid_from <= valid_to,
            "valid_from must be less than or equal to valid_to"
        );

        extra_holds.sort();
        extra_holds.dedup();
        extra_bizds.sort();
        extra_bizds.dedup();

        // check that extra_holds are weekdays
        ensure!(
            extra_holds
                .iter()
                .all(|d| d.weekday().number_from_monday() <= 5),
            "Extra holidays must be weekdays"
        );
        // check that extra_bizds are weekends
        ensure!(
            extra_bizds
                .iter()
                .all(|d| d.weekday().number_from_monday() > 5),
            "Extra business days must be weekends"
        );
        Ok(Self {
            extra_holds,
            extra_bizds,
            valid_from,
            valid_to,
        })
    }
}

// -----------------------------------------------------------------------------
// Calendar
//
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Calendar(Arc<CalendarData>);

//
// display, serde
//
impl Serialize for Calendar {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Calendar {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Calendar, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = CalendarData::deserialize(deserializer)?;
        Ok(Calendar(Arc::new(data)))
    }
}

//
// construction
//
impl Calendar {
    /// Create a new calendar with the given extra holidays and business days.
    ///
    /// As `extra_holds`, this function expects that days which are non-business day weekdays.
    /// As `extra_bizds`, this function expects that days which are business day weekends.
    ///
    /// `valid_from` and `valid_to` are the valid period of the calendar.
    /// The valid period is a half-open interval `[valid_from, valid_to)` and
    /// `valid_from <= valid_to` must hold.
    ///
    /// # Errors
    /// - If the given extra holidays are not weekdays
    /// - If the given extra business days are not weekends
    fn _new(
        extra_holds: Vec<NaiveDate>,
        extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
    ) -> anyhow::Result<Self> {
        CalendarData::new(extra_holds, extra_bizds, valid_from, valid_to)
            .map(Arc::new)
            .map(Self)
    }

    /// Create a new calendar builder with default values.
    pub fn builder() -> CalendarBuilder<(), (), ()> {
        CalendarBuilder::new()
    }

    /// Create a new calendar from multiple caneldars with `AnyClosed` strategy.
    /// This strategy considers a day as a holiday if it is a holiday in any of the given calendars.
    ///
    /// For example, if today is not a holiday of Tokyo but a holiday of New York,
    /// the day is considered as a holiday in the new calendar.
    pub fn of_any_closed<'a>(cals: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut extra_holds = HashSet::new();
        let mut extra_bizds: Option<HashSet<_>> = None;
        let mut valid_from = NaiveDate::MIN;
        let mut valid_to = NaiveDate::MAX;

        for cal in cals {
            extra_holds.extend(cal.0.extra_holds.iter().copied());

            match extra_bizds {
                None => extra_bizds = Some(cal.0.extra_bizds.iter().copied().collect()),
                Some(ref mut bizds) => {
                    bizds.retain(|d| cal.0.extra_bizds.contains(d));
                }
            }
            valid_from = valid_from.max(cal.0.valid_from);
            valid_to = valid_to.min(cal.0.valid_to);
        }
        Self::_new(
            extra_holds.into_iter().collect(),
            extra_bizds.into_iter().flatten().collect(),
            valid_from,
            valid_to,
        )
        .expect("AnyClosed of valid calendars must be valid")
    }

    /// Create a new calendar from multiple caneldars with `AllClosed` strategy.
    /// This strategy considers a day as a holiday only if it is a holiday in all of the given calendars.
    ///
    /// For example, if today is a holiday of Tokyo but not a holiday of New York,
    /// the day is considered as a business day in the new calendar.
    pub fn of_all_closed<'a>(cals: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut extra_holds: Option<HashSet<_>> = None;
        let mut extra_bizds = HashSet::new();
        let mut valid_from = NaiveDate::MIN;
        let mut valid_to = NaiveDate::MAX;

        for cal in cals {
            extra_bizds.extend(cal.0.extra_bizds.iter().copied());

            match extra_holds {
                None => extra_holds = Some(cal.0.extra_holds.iter().copied().collect()),
                Some(ref mut holds) => {
                    holds.retain(|d| cal.0.extra_holds.contains(d));
                }
            }
            valid_from = valid_from.max(cal.0.valid_from);
            valid_to = valid_to.min(cal.0.valid_to);
        }
        Self::_new(
            extra_holds.into_iter().flatten().collect(),
            extra_bizds.into_iter().collect(),
            valid_from,
            valid_to,
        )
        .expect("AllClosed of valid calendars must be valid")
    }
}

//
// methods
//
impl Calendar {
    /// Get the valid period of the calendar.
    ///
    /// Because we don't have infinitely many holidays and business days,
    /// some days are not supported by this calendar.
    ///
    /// This method returns the valid period of the calendar.
    /// The valid period is a half-open interval `[valid_from, valid_to)` and
    /// `valid_from <= valid_to` always holds.
    #[inline]
    pub fn valid_period(&self) -> (NaiveDate, NaiveDate) {
        (self.0.valid_from, self.0.valid_to)
    }

    /// Check if the given date is valid in the calendar.
    ///
    /// Because we don't have infinitely many holidays and business days,
    /// some days are not supported by this calendar.
    ///
    /// This method check that the given date is supported by the calendar.
    #[inline]
    pub fn does_support(&self, date: &NaiveDate) -> bool {
        &self.0.valid_from <= date && date < &self.0.valid_to
    }

    /// Get the extra holidays of the calendar.
    #[inline]
    pub fn extra_holidays(&self) -> &[NaiveDate] {
        &self.0.extra_holds
    }

    /// Get the extra business days of the calendar.
    #[inline]
    pub fn extra_business_days(&self) -> &[NaiveDate] {
        &self.0.extra_bizds
    }

    /// Check if the given date is a holiday.
    #[inline]
    pub fn is_holiday(&self, date: &NaiveDate) -> Option<bool> {
        if !self.does_support(date) {
            return None;
        }
        if 5 < date.weekday().number_from_monday() {
            return Some(self.0.extra_bizds.binary_search(date).is_err());
        } else {
            return Some(self.0.extra_holds.binary_search(date).is_ok());
        }
    }

    /// Check if the given date is a business day.
    #[inline]
    pub fn is_business_day(&self, date: &NaiveDate) -> Option<bool> {
        if !self.does_support(date) {
            return None;
        }
        if 5 < date.weekday().number_from_monday() {
            return Some(self.0.extra_bizds.binary_search(date).is_ok());
        } else {
            return Some(self.0.extra_holds.binary_search(date).is_err());
        }
    }

    /// Iterator over the business days from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Calendar;
    /// use chrono::NaiveDate;
    ///
    /// let cal = Calendar::new(
    ///     vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
    ///     vec![],
    ///     NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
    /// ).unwrap();
    ///
    /// let mut iter = cal.iter_bizdays(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    ///
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 4).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 6).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 7).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 8).unwrap()));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter_bizdays<'a>(
        &'a self,
        start: NaiveDate,
    ) -> impl DoubleEndedIterator<Item = NaiveDate> + 'a {
        DateIterator {
            cur: start,
            from: self.0.valid_from,
            to: self.0.valid_to,
        }
        .filter(move |d| self.is_business_day(d).unwrap_or(false))
    }

    /// Iterator over the holidays from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    ///
    /// # Example
    /// ```
    /// use qcore::chrono::Calendar;
    /// use chrono::NaiveDate;
    ///
    /// let cal = Calendar::new(
    ///     vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
    ///     vec![],
    ///     NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
    /// ).unwrap();
    ///
    /// let mut iter = cal.iter_holidays(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap()));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter_holidays<'a>(
        &'a self,
        start: NaiveDate,
    ) -> impl DoubleEndedIterator<Item = NaiveDate> + 'a {
        DateIterator {
            cur: start,
            from: self.0.valid_from,
            to: self.0.valid_to,
        }
        .filter(move |d| self.is_holiday(d).unwrap_or(false))
    }
}

//
// operators
//
impl BitAnd for Calendar {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::of_all_closed([self, rhs].iter())
    }
}

impl BitAnd<Calendar> for &Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: Calendar) -> Self::Output {
        Calendar::of_all_closed([self, &rhs])
    }
}

impl BitAnd for &Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: Self) -> Self::Output {
        Calendar::of_all_closed([self, rhs])
    }
}

impl BitAnd<&Calendar> for Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: &Self) -> Self::Output {
        Calendar::of_all_closed([&self, rhs])
    }
}

impl BitOr for Calendar {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::of_any_closed([self, rhs].iter())
    }
}

impl BitOr<Calendar> for &Calendar {
    type Output = Calendar;

    fn bitor(self, rhs: Calendar) -> Self::Output {
        Calendar::of_any_closed([self, &rhs])
    }
}

impl BitOr for &Calendar {
    type Output = Calendar;

    fn bitor(self, rhs: Self) -> Self::Output {
        Calendar::of_any_closed([self, rhs])
    }
}

impl BitOr<&Calendar> for Calendar {
    type Output = Calendar;

    fn bitor(self, rhs: &Self) -> Self::Output {
        Calendar::of_any_closed([&self, rhs])
    }
}

// -----------------------------------------------------------------------------
// DateIterator
//
struct DateIterator {
    cur: NaiveDate,
    from: NaiveDate,
    to: NaiveDate,
}

impl Iterator for DateIterator {
    type Item = NaiveDate;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.from || self.to <= self.cur {
            return None;
        }
        let ret = self.cur;
        self.cur = self.cur.checked_add_days(chrono::Days::new(1))?;
        Some(ret)
    }
}
impl DoubleEndedIterator for DateIterator {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.cur < self.from || self.to <= self.cur {
            return None;
        }
        let ret = self.cur;
        self.cur = self.cur.checked_sub_days(chrono::Days::new(1))?;
        Some(ret)
    }
}

// -----------------------------------------------------------------------------
// CalendarBuilder
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CalendarBuilder<H, B, V> {
    extra_holds: H,
    extra_bizds: B,
    valid_from: V,
    valid_to: V,
}

//
// construction
//
impl Default for CalendarBuilder<(), (), ()> {
    #[inline]
    fn default() -> Self {
        Self {
            extra_holds: (),
            extra_bizds: (),
            valid_from: (),
            valid_to: (),
        }
    }
}

impl CalendarBuilder<(), (), ()> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B, V> CalendarBuilder<(), B, V> {
    /// Set the extra holidays of the calendar.
    ///
    /// As `extra_holds`, this function expects that days which are non-business day weekdays.
    pub fn with_extra_holidays(
        self,
        extra_holds: Vec<NaiveDate>,
    ) -> CalendarBuilder<Vec<NaiveDate>, B, V> {
        CalendarBuilder {
            extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
        }
    }
}

impl<H, V> CalendarBuilder<H, (), V> {
    /// Set the extra business days of the calendar.
    ///
    /// As `extra_bizds`, this function expects that days which are business day weekends.
    pub fn with_extra_business_days(
        self,
        extra_bizds: Vec<NaiveDate>,
    ) -> CalendarBuilder<H, Vec<NaiveDate>, V> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
        }
    }
}

impl<H, B> CalendarBuilder<H, B, ()> {
    /// Set the valid period of the calendar.
    ///
    /// The valid period is a half-open interval `[valid_from, valid_to)` and
    /// `valid_from <= valid_to` must hold.
    pub fn with_valid_period(
        self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> CalendarBuilder<H, B, NaiveDate> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: from,
            valid_to: to,
        }
    }
}

impl CalendarBuilder<Vec<NaiveDate>, Vec<NaiveDate>, NaiveDate> {
    /// Build a new calendar.
    ///
    /// # Errors
    /// - If the given extra holidays are not weekdays
    /// - If the given extra business days are not weekends
    pub fn build(self) -> anyhow::Result<Calendar> {
        Calendar::_new(
            self.extra_holds,
            self.extra_bizds,
            self.valid_from,
            self.valid_to,
        )
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_new() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        );
        assert!(cal.is_ok());

        // duplicated extra holidays, unsorted extra holidays are allowed
        let cal = Calendar::_new(
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 5).unwrap(),
            ],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        );
        assert!(cal.is_ok());

        // invalid extra holidays
        let cal = Calendar::_new(
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
            ],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        );
        assert!(cal.is_err());

        // invalid extra business days
        let cal = Calendar::_new(
            vec![],
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
            ],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        );
        assert!(cal.is_err());

        // invalid valid period
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        );
        assert!(cal.is_err());
    }

    #[test]
    fn test_serialize() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&cal).unwrap();
        assert_eq!(
            json,
            r#"{"extra_holidays":["2021-01-01"],"extra_business_days":["2021-01-02"],"valid_from":"2021-01-01","valid_to":"2021-01-10"}"#
        );
    }

    #[test]
    fn test_deserialize() {
        let json = r#"{"extra_holidays":["2021-01-01"],"extra_business_days":["2021-01-02"],"valid_from":"2021-01-01","valid_to":"2021-01-10"}"#;
        let cal: Calendar = serde_json::from_str(json).unwrap();
        assert_eq!(
            cal.extra_holidays(),
            &[NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()]
        );
        assert_eq!(
            cal.extra_business_days(),
            &[NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()]
        );
        assert_eq!(
            cal.valid_period(),
            (
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()
            )
        );
    }

    #[test]
    fn test_of_any_closed() {
        let cal1 = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(),
            ],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        let cal = Calendar::of_any_closed(vec![&cal1, &cal2].into_iter());
        assert_eq!(
            cal.extra_holidays(),
            &[
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()
            ]
        );
        assert_eq!(
            cal.extra_business_days(),
            &[NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),]
        );
    }

    #[test]
    fn test_of_all_closed() {
        let cal1 = Calendar::_new(
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 5).unwrap(),
            ],
            vec![NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        let cal = Calendar::of_all_closed(vec![&cal1, &cal2].into_iter());
        assert_eq!(
            cal.extra_holidays(),
            &[NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()]
        );
        assert_eq!(
            cal.extra_business_days(),
            &[
                NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()
            ]
        );
    }

    #[test]
    fn test_valid_period() {
        let cal = Calendar::_new(
            vec![],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();
        assert_eq!(
            cal.valid_period(),
            (
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()
            )
        );
    }

    #[test]
    fn test_does_support() {
        let cal = Calendar::_new(
            vec![],
            vec![],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();
        assert!(!cal.does_support(&NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()));
        assert!(cal.does_support(&NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()));
        assert!(cal.does_support(&NaiveDate::from_ymd_opt(2021, 1, 9).unwrap()));
        assert!(!cal.does_support(&NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()));
        assert!(!cal.does_support(&NaiveDate::from_ymd_opt(2021, 1, 11).unwrap()));
    }

    #[test]
    fn test_is_holiday() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2020, 12, 30).unwrap()),
            None
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()),
            None
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 4).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 6).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 7).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 8).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 9).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_holiday(&NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()),
            None
        );
    }

    #[test]
    fn test_is_business_day() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 4).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 5).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 6).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 7).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 8).unwrap()),
            Some(true)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 9).unwrap()),
            Some(false)
        );
        assert_eq!(
            cal.is_business_day(&NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()),
            None
        );
    }

    #[test]
    fn test_iter_bizdays() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        let mut iter = cal.iter_bizdays(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 2).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 4).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 5).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 6).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 7).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 8).unwrap())
        );
        assert_eq!(iter.next(), None);

        // reverse
        let mut iter = cal
            .iter_bizdays(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap())
            .rev();
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 8).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 7).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 6).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 5).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 4).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 2).unwrap())
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_holidays() {
        let cal = Calendar::_new(
            vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()],
            vec![NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()],
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        )
        .unwrap();

        let mut iter = cal.iter_holidays(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 3).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap())
        );
        assert_eq!(iter.next(), None);

        // reverse
        let mut iter = cal
            .iter_holidays(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap())
            .rev();
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 3).unwrap())
        );
        assert_eq!(
            iter.next(),
            Some(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap())
        );
        assert_eq!(iter.next(), None);
    }
}
