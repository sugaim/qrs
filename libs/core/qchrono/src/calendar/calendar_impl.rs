use std::{
    ops::{BitAnd, BitOr, Bound, Range, RangeBounds},
    sync::Arc,
};

use anyhow::ensure;
use chrono::{Datelike, Days, NaiveDate, Weekday};

// -----------------------------------------------------------------------------
// _CalendarData
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, schemars::JsonSchema)]
struct _CalendarData {
    /// The extra holidays of the calendar. These days are non-business day weekdays
    /// if `treat_weekend_as_business_day` is `false`.
    #[serde(rename = "extra_holidays")]
    extra_holds: Vec<NaiveDate>,

    /// The extra business days of the calendar. These days are business day weekends.
    /// Must be empty if `treat_weekend_as_business_day` is `true`.
    #[serde(rename = "extra_business_days")]
    extra_bizds: Vec<NaiveDate>,

    /// The valid period of the calendar. include `valid_from`.
    valid_from: NaiveDate,

    /// The valid period of the calendar. exclude `valid_to`.
    valid_to: NaiveDate,

    /// Weekdays treated as holidays as default. Typically, Saturday and Sunday.
    holiday_weekdays: Vec<Weekday>,
}

//
// ser/de
//
impl<'de> serde::Deserialize<'de> for _CalendarData {
    fn deserialize<D>(deserializer: D) -> Result<_CalendarData, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct _Data {
            extra_holidays: Vec<NaiveDate>,
            extra_business_days: Vec<NaiveDate>,
            valid_from: NaiveDate,
            valid_to: NaiveDate,
            holiday_weekdays: Vec<Weekday>,
        }

        let data = _Data::deserialize(deserializer)?;
        _CalendarData::new(
            data.extra_holidays,
            data.extra_business_days,
            data.valid_from,
            data.valid_to,
            data.holiday_weekdays,
        )
        .map_err(serde::de::Error::custom)
    }
}

//
// ctor
//
impl _CalendarData {
    fn new(
        mut extra_holds: Vec<NaiveDate>,
        mut extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
        mut weekends: Vec<Weekday>,
    ) -> anyhow::Result<Self> {
        ensure!(
            valid_from < valid_to,
            "valid_from must be less than or equal to valid_to: valid_from={valid_from}, valid_to={valid_to}",
        );

        extra_holds.sort();
        extra_holds.dedup();
        extra_bizds.sort();
        extra_bizds.dedup();
        weekends.sort_by_key(|a| a.number_from_monday());
        weekends.dedup();

        ensure!(
            extra_bizds.iter().all(|d| weekends.contains(&d.weekday())),
            "Extra business days must be default holidays"
        );
        ensure!(
            extra_holds.iter().all(|d| !weekends.contains(&d.weekday())),
            "Extra holidays must be non-default holidays"
        );
        extra_holds.retain(|d| &valid_from <= d && d < &valid_to);
        extra_bizds.retain(|d| &valid_from <= d && d < &valid_to);

        Ok(Self {
            extra_holds,
            extra_bizds,
            valid_from,
            valid_to,
            holiday_weekdays: weekends,
        })
    }
}

// -----------------------------------------------------------------------------
// CalendarError
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Hash)]
pub enum CalendarError {
    #[error("{operation} does not suppoort unbounded range of dates")]
    Unbounded { operation: &'static str },
    #[error("The date {date} is out of the valid period [{}, {})", .valid_period.start, .valid_period.end)]
    OutOfValidPeriod {
        date: NaiveDate,
        valid_period: Range<NaiveDate>,
    },
}

// -----------------------------------------------------------------------------
// Calendar
// -----------------------------------------------------------------------------
/// Object manages business days and holidays
///
/// # Overview
/// This object manages business days and provides methods related to them,
/// such as checking if a given date is a holiday.
///
/// ```
/// use chrono::{NaiveDate, Weekday};
/// use qchrono::calendar::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| {
///     NaiveDate::from_ymd_opt(y, m, d).unwrap()
/// };
///
/// let cal = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
///     .build()
///     .unwrap();
///
/// // holiday check
/// assert!(cal.is_holiday(ymd(2021, 1, 1)).unwrap());   // New Year's Day
/// assert!(cal.is_holiday(ymd(2021, 1, 2)).unwrap());   // Saturday
/// assert!(cal.is_holiday(ymd(2021, 1, 3)).unwrap());   // Sunday
/// assert!(!cal.is_holiday(ymd(2021, 1, 4)).unwrap());  // Monday
///
/// // iteration over holidays
/// let mut iter = cal.iter_holidays(ymd(2021, 1, 1));
/// assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
/// assert_eq!(iter.next(), Some(ymd(2021, 1, 2)));
/// assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
/// assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
/// assert_eq!(iter.next(), None);
/// ```
///
/// Calendar implementation consists of the following four data.
/// - holiday weekdays: weekdays treated as holidays as default. Typically, Saturday and Sunday.
/// - extra holidays: non-business day which is not a holiday weekday
/// - extra business days: business day which is a holiday weekday
/// - valid period: the valid period of the calendar
///
/// # Combination of Calendars
/// Calendars can be combined in two manners
/// - any-closed strategy: a day is a holiday if it is a holiday in any of the given calendars
/// - all-closed strategy: a day is a holiday if it is a holiday day in all of the given calendars
///
/// These are implemented by [`Calendar::any_closed_of`] and [`Calendar::all_closed_of`] respectively.
///
/// When we focus on the set of holidays,
/// the any-closed strategy is equivalent to the union of holidays sets
/// and the all-closed strategy is equivalent to the intersection of holidays sets.
/// Hence, these are implemented by the [`BitOr`] and [`BitAnd`] operators respectively.
///
/// ```
/// use chrono::{NaiveDate, Weekday};
/// use qchrono::calendar::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| {
///     NaiveDate::from_ymd_opt(y, m, d).unwrap()
/// };
///
/// let cal1 = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
///     .build()
///     .unwrap();
///
/// let cal2 = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 5)])
///     .with_extra_business_days(vec![])
///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
///     .build()
///     .unwrap();
///
/// let cal = cal1 | cal2;
/// assert!(cal.is_holiday(ymd(2021, 1, 1)).unwrap());
/// assert!(cal.is_holiday(ymd(2021, 1, 5)).unwrap());
/// ```
///
/// # Lightweight
/// [`Calendar`] contains some vectors, it is rarely to modify them and we need clone them frequently.
/// So, the internal data is wrapped by immutable [`Arc`] and the object is lightweight.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Calendar(Arc<_CalendarData>);

//
// ser/de
//
impl serde::Serialize for Calendar {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Calendar {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Calendar, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = _CalendarData::deserialize(deserializer)?;
        Ok(Calendar(Arc::new(data)))
    }
}

impl schemars::JsonSchema for Calendar {
    fn schema_name() -> String {
        "Calendar".to_string()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qchrono::calendar::Calendar".into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <_CalendarData as schemars::JsonSchema>::json_schema(gen)
    }
}

//
// construction
//
impl Calendar {
    fn _new(
        extra_holds: Vec<NaiveDate>,
        extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
        holiday_weekdays: Vec<Weekday>,
    ) -> anyhow::Result<Self> {
        _CalendarData::new(
            extra_holds,
            extra_bizds,
            valid_from,
            valid_to,
            holiday_weekdays,
        )
        .map(Arc::new)
        .map(Self)
    }

    /// Get [CalendarBuilder] instance.
    #[inline]
    pub fn builder() -> CalendarBuilder {
        CalendarBuilder::new()
    }

    /// Create a new calendar from multiple caneldars with any-closed strategy.
    /// With this strategy, a day is a holiday if it is a holiday in any of the given calendars.
    ///
    /// For example, if today is not a holiday of Tokyo but a holiday of New York,
    /// the day is considered as a holiday in the combined calendar.
    ///
    /// This function requires an iterator over values, not references.
    /// Because the [`Calendar`] object is lightweight, please clone objects if necessary.
    ///
    /// When given iterator is empty or no ovarlapping valid period, [None] is returned.
    #[inline]
    pub fn any_closed_of<It>(cals: It) -> Option<Self>
    where
        It: IntoIterator<Item = Self>,
    {
        let cals = smallvec::SmallVec::<[Self; 10]>::from_iter(cals);
        match cals.len() {
            0 => return None,
            1 => return Some(cals.into_iter().next().unwrap()),
            _ => {}
        }

        let valid_per = {
            let valid_from = *cals.iter().map(|c| &c.0.valid_from).max()?;
            let valid_to = *cals.iter().map(|c| &c.0.valid_to).min()?;
            valid_from..valid_to
        };
        if valid_per.is_empty() {
            return None;
        }

        let hol_wds = {
            let hol_wds = cals.iter().flat_map(|c| &c.0.holiday_weekdays);
            let mut hol_wds = hol_wds.copied().collect::<Vec<_>>();
            hol_wds.sort_by_key(|a| a.number_from_monday());
            hol_wds.dedup();
            hol_wds
        };

        let extra_holds = {
            let holds = cals.iter().flat_map(|c| &c.0.extra_holds);
            let holds = holds.filter(|d| valid_per.contains(d) && !hol_wds.contains(&d.weekday()));
            let mut holds = holds.copied().collect::<Vec<_>>();
            holds.sort();
            holds.dedup();
            holds
        };

        let extra_bizds = {
            let bizds = cals.iter().flat_map(|c| &c.0.extra_bizds);
            let bizds = bizds.filter(|d| cals.iter().all(|c| c.is_bizday(**d).unwrap_or(false)));
            let mut bizds = bizds.copied().collect::<Vec<_>>();
            bizds.sort();
            bizds.dedup();
            bizds
        };

        Self::_new(
            extra_holds,
            extra_bizds,
            valid_per.start,
            valid_per.end,
            hol_wds,
        )
        .expect("AnyClosed of valid calendars must be valid")
        .into()
    }

    /// Create a new calendar from multiple caneldars with all-closed strategy.
    /// With this strategy, a day is a holiday if it is a holiday day in all of the given calendars.
    ///
    /// For example, if today is a holiday of Tokyo but not a holiday of New York,
    /// the day is considered as a business day in the combined calendar.
    ///
    /// This function requires an iterator over values, not references.
    /// Because the [`Calendar`] object is lightweight, please clone objects if necessary.
    ///
    /// When given iterator is empty or no ovarlapping valid period, [None] is returned.
    #[inline]
    pub fn all_closed_of<It>(cals: It) -> Option<Self>
    where
        It: IntoIterator<Item = Self>,
    {
        let cals = smallvec::SmallVec::<[Self; 10]>::from_iter(cals);
        match cals.len() {
            0 => return None,
            1 => return Some(cals.into_iter().next().unwrap()),
            _ => {}
        }

        let valid_per = {
            let valid_from = *cals.iter().map(|c| &c.0.valid_from).max()?;
            let valid_to = *cals.iter().map(|c| &c.0.valid_to).min()?;
            valid_from..valid_to
        };
        if valid_per.is_empty() {
            return None;
        }

        let hol_wds = {
            let hol_wds = cals.iter().flat_map(|c| &c.0.holiday_weekdays);
            let hol_wds = hol_wds.filter(|w| cals.iter().all(|c| c.holiday_weekdays().contains(w)));
            let mut hol_wds = hol_wds.copied().collect::<Vec<_>>();
            hol_wds.sort_by_key(|a| a.number_from_monday());
            hol_wds.dedup();
            hol_wds
        };

        let extra_holds = {
            let holds = cals.iter().flat_map(|c| &c.0.extra_holds);
            let holds = holds.filter(|d| cals.iter().all(|c| c.is_holiday(**d).unwrap_or(false)));
            let mut holds = holds.copied().collect::<Vec<_>>();
            holds.sort();
            holds.dedup();
            holds
        };

        let extra_bizds = {
            let bizds = cals.iter().flat_map(|c| &c.0.extra_bizds);
            let bizds = bizds.filter(|d| hol_wds.contains(&d.weekday()));
            let mut bizds = bizds.copied().collect::<Vec<_>>();
            bizds.sort();
            bizds.dedup();
            bizds
        };

        Self::_new(
            extra_holds,
            extra_bizds,
            valid_per.start,
            valid_per.end,
            hol_wds,
        )
        .expect("AllClosed of valid calendars must be valid")
        .into()
    }
}

//
// methods
//
impl Calendar {
    /// Get the valid period of the calendar.
    ///
    /// Because we can't have infinitely many holidays and business days,
    /// some days are not supported by this calendar.
    ///
    /// This method returns the valid period of the calendar.
    /// The valid period is a half-open interval `valid_from..valid_to`
    /// where `valid_from < valid_to` always holds.
    #[inline]
    pub fn valid_period(&self) -> Range<NaiveDate> {
        self.0.valid_from..self.0.valid_to
    }

    /// Check if the given date is supported by the calendar.
    #[inline]
    fn is_suppoted(&self, date: NaiveDate) -> Result<NaiveDate, CalendarError> {
        if !self.valid_period().contains(&date) {
            Err(CalendarError::OutOfValidPeriod {
                date,
                valid_period: self.valid_period(),
            })
        } else {
            Ok(date)
        }
    }

    /// Get the extra holidays of the calendar.
    #[inline]
    pub fn extra_holidays(&self) -> &[NaiveDate] {
        &self.0.extra_holds
    }

    /// Get the extra business days of the calendar.
    #[inline]
    pub fn extra_bizdays(&self) -> &[NaiveDate] {
        &self.0.extra_bizds
    }

    /// Weekdays treated as holidays as default.
    #[inline]
    pub fn holiday_weekdays(&self) -> &[Weekday] {
        &self.0.holiday_weekdays
    }

    /// Count the business days between the given range
    /// When the range is empty, this method returns `0`.
    ///
    /// # Errors
    /// * [`CalendarError::Unbounded`]: When the range is unbounded
    /// * [`CalendarError::OutOfValidPeriod`]: When the range contains a date which is out of the valid period
    ///
    /// # Example
    /// ```
    /// use chrono::{NaiveDate, Weekday};
    /// use qchrono::calendar::Calendar;
    ///
    /// let cal = Calendar::builder()
    ///     .with_valid_period(NaiveDate::MIN, NaiveDate::MAX)
    ///     .with_extra_holidays(vec![])
    ///     .with_extra_business_days(vec![])
    ///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
    ///     .build()
    ///     .unwrap();
    ///
    /// let from = NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(); // Sun
    /// let to = NaiveDate::from_ymd_opt(2021, 1, 8).unwrap(); // Fri
    ///
    /// assert_eq!(cal.num_bizdays(from..to), Ok(4)); // Mon, Tue, Wed, Thu
    /// assert_eq!(cal.num_bizdays(from..=to), Ok(5)); // Mon, Tue, Wed, Thu, Fri
    ///
    /// // zero is returned for empty range
    /// assert_eq!(cal.num_bizdays(to..from), Ok(0));
    ///
    /// // unbounded range is not supported
    /// assert!(cal.num_bizdays(from..).is_err());
    /// assert!(cal.num_bizdays(..to).is_err());
    /// assert!(cal.num_bizdays(..).is_err());
    /// ```
    pub fn num_bizdays<R>(&self, range: R) -> Result<usize, CalendarError>
    where
        R: RangeBounds<NaiveDate>,
    {
        // treat trivial cases, unbounded or empty range
        match (range.start_bound(), range.end_bound()) {
            (Bound::Unbounded, _) | (_, Bound::Unbounded) => {
                return Err(CalendarError::Unbounded {
                    operation: "counting business days",
                })
            }
            (Bound::Included(&s), Bound::Included(&e)) if s > e => return Ok(0),
            (Bound::Included(&s), Bound::Excluded(&e)) if s >= e => return Ok(0),
            (Bound::Excluded(&s), Bound::Included(&e)) if s >= e => return Ok(0),
            (Bound::Excluded(&s), Bound::Excluded(&e)) if s >= e => return Ok(0),
            _ => {}
        };

        // adjust range to half-open interval
        // to count the days between [start, end).
        let start = match range.start_bound() {
            Bound::Unbounded => unreachable!(),
            Bound::Included(&d) => self.is_suppoted(d)?,
            Bound::Excluded(&d) => {
                self.is_suppoted(d.checked_add_days(Days::new(1)).ok_or_else(|| {
                    CalendarError::OutOfValidPeriod {
                        date: d,
                        valid_period: self.valid_period(),
                    }
                })?)?
            }
        };
        let end = match range.end_bound() {
            Bound::Unbounded => unreachable!(),
            Bound::Included(&d) => self
                .is_suppoted(d)?
                .checked_add_days(Days::new(1))
                .ok_or_else(|| CalendarError::OutOfValidPeriod {
                    date: d,
                    valid_period: self.valid_period(),
                })?,
            Bound::Excluded(&d) => {
                self.is_suppoted(d.checked_sub_days(Days::new(1)).ok_or_else(|| {
                    CalendarError::OutOfValidPeriod {
                        date: d,
                        valid_period: self.valid_period(),
                    }
                })?)?;
                d
            }
        };

        let extra_hols = {
            let stt = self.extra_holidays().partition_point(|d| *d < start);
            let end = self.extra_holidays().partition_point(|d| *d < end);
            (end - stt) as i64
        };
        let extra_bds = {
            let stt = self.extra_bizdays().partition_point(|d| *d < start);
            let end = self.extra_bizdays().partition_point(|d| *d < end);
            (end - stt) as i64
        };

        let mut sub_wds = 0;
        let mut cur_wd = start.weekday();
        while cur_wd != end.weekday() {
            if !self.holiday_weekdays().contains(&cur_wd) {
                sub_wds += 1;
            }
            cur_wd = cur_wd.succ();
        }
        let num_wds_in_week = 7 - self.holiday_weekdays().len() as i64;
        let naive_count = (end - start).num_days() / 7 * num_wds_in_week + sub_wds;

        Ok((naive_count - extra_hols + extra_bds) as usize)
    }

    /// Check if the given date is a holiday.
    ///
    /// If the given date is not supported by the calendar, this method returns [`Err`].
    #[inline]
    pub fn is_holiday(&self, date: NaiveDate) -> Result<bool, CalendarError> {
        let date = self.is_suppoted(date)?;
        if self.holiday_weekdays().contains(&date.weekday()) {
            Ok(self.0.extra_bizds.binary_search(&date).is_err())
        } else {
            Ok(self.0.extra_holds.binary_search(&date).is_ok())
        }
    }

    /// Check if the given date is a business day.
    ///
    /// If the given date is not supported by the calendar, this method returns [`Err`].
    #[inline]
    pub fn is_bizday(&self, date: NaiveDate) -> Result<bool, CalendarError> {
        let date = self.is_suppoted(date)?;
        if self.holiday_weekdays().contains(&date.weekday()) {
            Ok(self.0.extra_bizds.binary_search(&date).is_ok())
        } else {
            Ok(self.0.extra_holds.binary_search(&date).is_err())
        }
    }

    /// Iterator over the business days from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    /// The first date of the iterator is the given date if it is a business day.
    ///
    /// # Example
    /// ```
    /// use chrono::{NaiveDate, Weekday};
    /// use qchrono::calendar::Calendar;
    ///
    /// let ymd = |y: i32, m: u32, d: u32| {
    ///    NaiveDate::from_ymd_opt(y, m, d).unwrap()
    /// };
    ///
    /// let cal = Calendar::builder()
    ///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
    ///     .with_extra_holidays(vec![ymd(2021, 1, 6)])
    ///     .with_extra_business_days(vec![])
    ///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut iter = cal.iter_bizdays(ymd(2021, 1, 1));
    ///
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 4)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 5)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 7)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 8)));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter_bizdays(
        &self,
        start: NaiveDate,
    ) -> impl DoubleEndedIterator<Item = NaiveDate> + '_ {
        DateIterator {
            cur: start,
            from: self.0.valid_from,
            to: self.0.valid_to,
        }
        .filter(move |d| self.is_bizday(*d).unwrap_or(false))
    }

    /// Iterator over the holidays from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    /// The first date of the iterator is the given date if it is a holiday.
    ///
    /// # Example
    /// ```
    /// use chrono::{NaiveDate, Weekday};
    /// use qchrono::calendar::Calendar;
    ///
    /// let ymd = |y: i32, m: u32, d: u32| {
    ///     NaiveDate::from_ymd_opt(y, m, d).unwrap()
    /// };
    ///
    /// let cal = Calendar::builder()
    ///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
    ///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
    ///     .with_extra_business_days(vec![])
    ///     .with_holiday_weekdays(vec![Weekday::Sat, Weekday::Sun])
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut iter = cal.iter_holidays(ymd(2021, 1, 1));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 2)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
    /// assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter_holidays(
        &self,
        start: NaiveDate,
    ) -> impl DoubleEndedIterator<Item = NaiveDate> + '_ {
        DateIterator {
            cur: start,
            from: self.0.valid_from,
            to: self.0.valid_to,
        }
        .filter(move |d| self.is_holiday(*d).unwrap_or(false))
    }
}

//
// operators
//
impl BitAnd for Calendar {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::all_closed_of([self, rhs]).expect("`Some` for non-empty iterator")
    }
}

impl BitOr for Calendar {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::any_closed_of([self, rhs]).expect("`Some` for non-empty iterator")
    }
}

// -----------------------------------------------------------------------------
// DateIterator
// -----------------------------------------------------------------------------
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
// -----------------------------------------------------------------------------
/// Builder of a calendar
///
/// The [`Calendar`] consists of the three data, extra holidays, extra business days, and valid period.
/// (See the documentation of [`Calendar`] for more details)
///
/// This builder provides methods to set these data and build a new calendar.
/// Weekdays treated as holidays can be controlled by the method [`CalendarBuilder::with_holiday_weekdays`].
/// For typical calendars which treat Saturday and Sunday as holidays,
/// please set [`Weekday::Sat`] and [`Weekday::Sun`] to this method.
///
/// This builder has type parameters for each data.
/// These are used to control builder methods and prevent multiple calls of the same method.
///
/// # Example
/// ```
/// use chrono::NaiveDate;
/// use qchrono::calendar::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| {
///     NaiveDate::from_ymd_opt(y, m, d).unwrap()
/// };
///
/// let cal = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .with_holiday_weekdays(vec![])
///     .build();
///
/// ````
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CalendarBuilder<H = (), B = (), V = (), W = ()> {
    extra_holds: H,
    extra_bizds: B,
    valid_from: V,
    valid_to: V,
    holiday_weekdays: W,
}

//
// construction
//
impl Default for CalendarBuilder {
    #[inline]
    fn default() -> Self {
        Self {
            extra_holds: (),
            extra_bizds: (),
            valid_from: (),
            valid_to: (),
            holiday_weekdays: (),
        }
    }
}

impl CalendarBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B, V, W> CalendarBuilder<(), B, V, W> {
    /// Set the extra holidays of the calendar.
    ///
    /// As `extra_holds`, this function expects that days which are non-business day weekdays.
    pub fn with_extra_holidays(
        self,
        extra_holds: Vec<NaiveDate>,
    ) -> CalendarBuilder<Vec<NaiveDate>, B, V, W> {
        CalendarBuilder {
            extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            holiday_weekdays: self.holiday_weekdays,
        }
    }
}

impl<H, V, W> CalendarBuilder<H, (), V, W> {
    /// Set the extra business days of the calendar.
    ///
    /// As `extra_bizds`, this function expects that days which are business day weekends.
    pub fn with_extra_business_days(
        self,
        extra_bizds: Vec<NaiveDate>,
    ) -> CalendarBuilder<H, Vec<NaiveDate>, V, W> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            holiday_weekdays: self.holiday_weekdays,
        }
    }
}

impl<H, B, W> CalendarBuilder<H, B, (), W> {
    /// Set the valid period of the calendar.
    ///
    /// The valid period is a half-open interval `[valid_from, valid_to)` and
    /// `valid_from < valid_to` must hold.
    pub fn with_valid_period(
        self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> CalendarBuilder<H, B, NaiveDate, W> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: from,
            valid_to: to,
            holiday_weekdays: self.holiday_weekdays,
        }
    }
}

impl<H, B, V> CalendarBuilder<H, B, V, ()> {
    /// Set the weekdays treated as holidays as default.
    pub fn with_holiday_weekdays(
        self,
        holiday_weekdays: Vec<Weekday>,
    ) -> CalendarBuilder<H, B, V, Vec<Weekday>> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            holiday_weekdays,
        }
    }
}

impl CalendarBuilder<Vec<NaiveDate>, Vec<NaiveDate>, NaiveDate, Vec<Weekday>> {
    /// Build a new calendar from the given data.
    ///
    /// # Errors
    /// - If the given extra holidays are holiday weekdays
    /// - If the given extra business days are not holiday weekdays
    /// - If the valid period is invalid (valie_to <= valid_from)
    pub fn build(self) -> anyhow::Result<Calendar> {
        Calendar::_new(
            self.extra_holds,
            self.extra_bizds,
            self.valid_from,
            self.valid_to,
            self.holiday_weekdays,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use rstest::rstest;

    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_new_ok() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_ok());
    }

    #[test]
    fn test_new_ok_dup() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sun, Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_ok());
    }

    #[test]
    fn test_new_ok_unsorted() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 5), ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_ok());
    }

    #[test]
    fn test_new_ng_weekend_extra_hol() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_err());
    }

    #[test]
    fn test_new_ok_weekend_extra_hol_with_treat_weekend_as_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        );

        assert!(cal.is_ok());
    }

    #[test]
    fn test_new_ng_weekday_extra_bd() {
        let cal = Calendar::_new(
            vec![],
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_err());
    }

    #[test]
    fn test_new_ng_unsorted_period() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 10),
            ymd(2021, 1, 1),
            vec![Weekday::Sat, Weekday::Sun],
        );

        assert!(cal.is_err());
    }

    #[test]
    fn test_new_ng_extra_bd_with_treat_weekend_as_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        );

        assert!(cal.is_err());
    }

    #[test]
    fn test_serialize() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let json = serde_json::to_value(&cal).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "extra_holidays": ["2021-01-01"],
                "extra_business_days": ["2021-01-02"],
                "valid_from": "2021-01-01",
                "valid_to": "2021-01-10",
                "holiday_weekdays": ["Sat", "Sun"]
            })
        );
    }

    #[test]
    fn test_deserialize() {
        let json = serde_json::json!({
            "extra_holidays": ["2021-01-01"],
            "extra_business_days": ["2021-01-02"],
            "valid_from": "2021-01-01",
            "valid_to": "2021-01-10",
            "holiday_weekdays": ["Sat", "Sun"]
        });

        let cal: Calendar = serde_json::from_value(json).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2)]);
        assert_eq!(cal.holiday_weekdays(), &[Weekday::Sat, Weekday::Sun]);
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
    }

    #[test]
    fn test_deserialize_treat_weekend_as_business_day() {
        let json = serde_json::json!({
            "extra_holidays": ["2021-01-01"],
            "extra_business_days": [],
            "valid_from": "2021-01-01",
            "valid_to": "2021-01-10",
            "holiday_weekdays": []
        });

        let cal: Calendar = serde_json::from_value(json).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(cal.holiday_weekdays().is_empty());
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
    }

    #[test]
    fn test_of_any_closed_empty() {
        let cal = Calendar::any_closed_of([]);

        assert!(cal.is_none());
    }

    #[test]
    fn test_of_any_closed_single() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::any_closed_of([cal1.clone()]).unwrap();

        assert_eq!(cal, cal1);
    }

    #[test]
    fn test_of_any_closed_single_treat_weekend_as_business_day() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();

        let cal = Calendar::any_closed_of([cal1.clone()]).unwrap();

        assert_eq!(cal, cal1);
    }

    #[test]
    fn test_of_any_closed_multiple() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2), ymd(2021, 1, 3)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::any_closed_of([cal1, cal2]).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2),]);
        assert!(!cal.holiday_weekdays().is_empty());
    }

    #[test]
    fn test_of_any_closed_multiple_treat_weekend_as_bizday() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::any_closed_of([cal1, cal2]).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2),]);
        assert_eq!(cal.holiday_weekdays(), &[Weekday::Sat, Weekday::Sun]);
    }

    #[test]
    fn test_of_any_closed_multiple_treat_weekend_as_bizday_all() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();

        let cal = Calendar::any_closed_of([cal1, cal2]).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(cal.holiday_weekdays().is_empty());
    }

    #[test]
    fn test_of_all_closed_empty() {
        let cal = Calendar::all_closed_of([]);

        assert!(cal.is_none());
    }

    #[test]
    fn test_of_all_closed_single() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::all_closed_of([cal1.clone()]).unwrap();

        assert_eq!(cal, cal1);
    }

    #[test]
    fn test_of_all_closed_single_treat_weekend_as_business_day() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();

        let cal = Calendar::all_closed_of([cal1.clone()]).unwrap();

        assert_eq!(cal, cal1);
    }

    #[test]
    fn test_of_all_closed_multiple() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2), ymd(2021, 1, 3)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::all_closed_of([cal1, cal2]).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2), ymd(2021, 1, 3)]);
        assert_eq!(cal.holiday_weekdays(), &[Weekday::Sat, Weekday::Sun]);
    }

    #[test]
    fn test_of_all_closed_multiple_treat_weekend_as_bizday() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = Calendar::all_closed_of([cal1, cal2]).unwrap();

        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert_eq!(cal.holiday_weekdays(), &[]);
    }

    #[test]
    fn test_valid_period() {
        let cal = Calendar::_new(
            vec![],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
    }

    #[test]
    fn test_validate() {
        let cal = Calendar::_new(
            vec![],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        assert!(cal.is_suppoted(ymd(2020, 12, 31)).is_err());
        assert!(cal.is_suppoted(ymd(2021, 1, 1)).is_ok());
        assert!(cal.is_suppoted(ymd(2021, 1, 9)).is_ok());
        assert!(cal.is_suppoted(ymd(2021, 1, 10)).is_err());
        assert!(cal.is_suppoted(ymd(2021, 1, 11)).is_err());
    }

    #[test]
    fn test_is_holiday() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        assert!(cal.is_holiday(ymd(2020, 12, 30)).is_err());
        assert!(cal.is_holiday(ymd(2020, 12, 31)).is_err());
        assert!(cal.is_holiday(ymd(2021, 1, 1)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 2)).unwrap());
        assert!(cal.is_holiday(ymd(2021, 1, 3)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 4)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 5)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 6)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 7)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 8)).unwrap());
        assert!(cal.is_holiday(ymd(2021, 1, 9)).unwrap());
        assert!(cal.is_holiday(ymd(2021, 1, 10)).is_err());
    }

    #[test]
    fn test_is_holiday_treat_weekend_as_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();

        assert!(cal.is_holiday(ymd(2020, 12, 30)).is_err());
        assert!(cal.is_holiday(ymd(2020, 12, 31)).is_err());
        assert!(cal.is_holiday(ymd(2021, 1, 1)).unwrap());
        assert!(cal.is_holiday(ymd(2021, 1, 2)).unwrap());
        assert!(!cal.is_holiday(ymd(2021, 1, 3)).unwrap()); // Sunday
        assert!(cal.is_holiday(ymd(2021, 1, 10)).is_err());
    }

    #[test]
    fn test_is_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        assert!(!cal.is_bizday(ymd(2021, 1, 1)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 2)).unwrap());
        assert!(!cal.is_bizday(ymd(2021, 1, 3)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 4)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 5)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 6)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 7)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 8)).unwrap());
        assert!(!cal.is_bizday(ymd(2021, 1, 9)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 10)).is_err());
    }

    #[test]
    fn test_is_business_day_treat_weekend_as_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        )
        .unwrap();

        assert!(!cal.is_bizday(ymd(2021, 1, 1)).unwrap());
        assert!(!cal.is_bizday(ymd(2021, 1, 2)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 3)).unwrap());
        assert!(cal.is_bizday(ymd(2021, 1, 10)).is_err());
    }

    #[test]
    fn test_iter_bizdays() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let mut iter = cal.iter_bizdays(ymd(2021, 1, 1));

        assert_eq!(iter.next(), Some(ymd(2021, 1, 2)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 4)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 5)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 6)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 7)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 8)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_bizdays_rev() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let mut iter = cal.iter_bizdays(ymd(2021, 1, 9)).rev();

        assert_eq!(iter.next(), Some(ymd(2021, 1, 8)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 7)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 6)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 5)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 4)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 2)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_holidays() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let mut iter = cal.iter_holidays(ymd(2021, 1, 1));

        assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iter_holidays_rev() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let mut iter = cal.iter_holidays(ymd(2021, 1, 9)).rev();

        assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
        assert_eq!(iter.next(), None);
    }

    #[rstest_reuse::template]
    #[rstest]
    #[case(
        Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        ).unwrap()
    )]
    #[case(
        Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            vec![],
        ).unwrap()
    )]
    #[case(
        Calendar::_new(
            vec![],
            vec![],
            NaiveDate::MIN,
            NaiveDate::MIN.checked_add_days(Days::new(100)).unwrap(),
            vec![Weekday::Sat, Weekday::Sun],
        ).unwrap()
    )]
    #[case(
        Calendar::_new(
            vec![],
            vec![],
            NaiveDate::MAX.checked_sub_days(Days::new(100)).unwrap(),
            NaiveDate::MAX,
            vec![Weekday::Sat, Weekday::Sun],
        ).unwrap()
    )]
    fn calendar_template(#[case] cal: Calendar) {}

    #[rstest_reuse::apply(calendar_template)]
    fn test_num_bizdays_unbounded(cal: Calendar) {
        let unbounded = cal.num_bizdays(..);

        assert!(matches!(unbounded, Err(CalendarError::Unbounded { .. })));
    }

    #[rstest_reuse::apply(calendar_template)]
    fn test_num_bizdays_unbounded_partial(
        cal: Calendar,
        #[values(
            NaiveDate::MIN,
            ymd(1999, 1, 1),
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            ymd(2021, 1, 13),
            ymd(2021, 1, 20),
            ymd(2025, 1, 1),
            NaiveDate::MAX
        )]
        d: NaiveDate,
    ) {
        let end_unbounded = cal.num_bizdays(d..);
        let stt_unbounded_end_incl = cal.num_bizdays(..=d);
        let stt_unbounded_end_excl = cal.num_bizdays(..d);

        assert!(matches!(
            end_unbounded,
            Err(CalendarError::Unbounded { .. })
        ));
        assert!(matches!(
            stt_unbounded_end_incl,
            Err(CalendarError::Unbounded { .. })
        ));
        assert!(matches!(
            stt_unbounded_end_excl,
            Err(CalendarError::Unbounded { .. })
        ));
    }

    #[rstest_reuse::apply(calendar_template)]
    fn test_num_bizdays(
        cal: Calendar,
        #[values(
            NaiveDate::MIN,
            ymd(1999, 1, 1),
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            ymd(2021, 1, 13),
            ymd(2021, 1, 20),
            ymd(2025, 1, 1),
            NaiveDate::MAX
        )]
        stt: NaiveDate,
        #[values(
            NaiveDate::MIN,
            ymd(1999, 1, 1),
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            ymd(2021, 1, 13),
            ymd(2021, 1, 20),
            ymd(2025, 1, 1),
            NaiveDate::MAX
        )]
        end: NaiveDate,
    ) {
        let excl_exp = stt
            .iter_days()
            .take_while(|d| d < &end)
            .map(|d| cal.is_bizday(d))
            .collect::<Result<Vec<_>, _>>()
            .map(|bs| bs.into_iter().filter(|b| *b).count())
            .ok();

        let incl_exp = if stt == NaiveDate::MAX && stt == end {
            // chrono::NaiveDate::iter_days can not treat NaiveDate::MAX
            None
        } else {
            stt.iter_days()
                .take_while(|d| d <= &end)
                .map(|d| cal.is_bizday(d))
                .collect::<Result<Vec<_>, _>>()
                .map(|bs| bs.into_iter().filter(|b| *b).count())
                .ok()
        };

        let excl = cal.num_bizdays(stt..end);
        let incl = cal.num_bizdays(stt..=end);

        assert_eq!(excl.ok(), excl_exp);
        assert_eq!(incl.ok(), incl_exp);
    }

    #[test]
    fn test_bitor() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2), ymd(2021, 1, 3)],
            ymd(2020, 12, 31),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 15),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = cal1.clone() | cal2.clone();

        assert_eq!(cal, Calendar::any_closed_of([cal1, cal2]).unwrap());
    }

    #[test]
    fn test_bitand() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 3)],
            ymd(2020, 12, 31),
            ymd(2021, 1, 10),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 15),
            vec![Weekday::Sat, Weekday::Sun],
        )
        .unwrap();

        let cal = cal1.clone() & cal2.clone();

        assert_eq!(cal, Calendar::all_closed_of([cal1, cal2]).unwrap());
    }
}
