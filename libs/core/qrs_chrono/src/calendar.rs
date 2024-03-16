use std::{
    borrow::Cow,
    collections::HashSet,
    ops::{BitAnd, BitOr, Range},
    sync::Arc,
};

use anyhow::ensure;
use chrono::{Datelike, NaiveDate};

// -----------------------------------------------------------------------------
// _CalendarData
//
/// Calendar data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, schemars::JsonSchema))]
struct _CalendarData {
    /// The extra holidays of the calendar. These days are non-business day weekdays
    /// if `treat_weekend_as_business_day` is `false`.
    #[cfg_attr(feature = "serde", serde(rename = "extra_holidays"))]
    extra_holds: Vec<NaiveDate>,

    /// The extra business days of the calendar. These days are business day weekends.
    /// Must be empty if `treat_weekend_as_business_day` is `true`.
    #[cfg_attr(feature = "serde", serde(rename = "extra_business_days"))]
    extra_bizds: Vec<NaiveDate>,

    /// The valid period of the calendar. include `valid_from`.
    valid_from: NaiveDate,

    /// The valid period of the calendar. exclude `valid_to`.
    valid_to: NaiveDate,

    /// Flag to treat weekend as business day
    #[cfg_attr(feature = "serde", serde(default))]
    treat_weekend_as_business_day: bool,
}

//
// display, serde
//
#[cfg(feature = "serde")]
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
            treat_weekend_as_business_day: Option<bool>,
        }

        let data = _Data::deserialize(deserializer)?;
        _CalendarData::new(
            data.extra_holidays,
            data.extra_business_days,
            data.valid_from,
            data.valid_to,
            data.treat_weekend_as_business_day.unwrap_or_default(),
        )
        .map_err(serde::de::Error::custom)
    }
}

//
// construction
//
impl _CalendarData {
    fn new(
        mut extra_holds: Vec<NaiveDate>,
        mut extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
        treat_weekend_as_business_day: bool,
    ) -> anyhow::Result<Self> {
        ensure!(
            valid_from <= valid_to,
            "valid_from must be less than or equal to valid_to"
        );

        extra_holds.sort();
        extra_holds.dedup();
        extra_bizds.sort();
        extra_bizds.dedup();

        // check that extra business days are empty
        // when weekends are treated as business days
        ensure!(
            !treat_weekend_as_business_day || extra_bizds.is_empty(),
            "Extra business days must be empty when treat_weekend_as_business_day is true"
        );
        // check that extra_holds are weekdays
        ensure!(
            treat_weekend_as_business_day
                || extra_holds
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
            treat_weekend_as_business_day,
        })
    }
}

// -----------------------------------------------------------------------------
// Calendar
//

/// Calendar object which manages business days
///
/// # Overview
/// This object manages business days and
/// provides methods to check if the given date is a holiday or a business day.
///
/// ```
/// use qrs_chrono::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
///
/// let cal = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .build()
///     .unwrap();
///
/// assert!(cal.is_holiday(&ymd(2021, 1, 1)).unwrap());  // New Year's Day
/// assert!(cal.is_holiday(&ymd(2021, 1, 2)).unwrap());  // Saturday
/// assert!(cal.is_holiday(&ymd(2021, 1, 3)).unwrap());  // Sunday
/// assert!(!cal.is_bizday(&ymd(2021, 1, 3)).unwrap());  // Monday
/// ```
///
/// As default, the Saturday and Sunday are considered as holidays
/// and calendar consists of the following three data to reduce data size
/// (These data are wrapped by [`Arc`] to cloning the calendar object efficiently)
/// - extra holidays: weekdays which are non-business day
/// - extra business days: weekends which are business day
/// - valid period: the valid period of the calendar
///
/// Extra holidays must be weekdays when the flag `treat_weekend_as_business_day` is `false`.
/// Extra business days always must be weekends.
///
/// To treat the weekend as business day, set the flag `treat_weekend_as_business_day` to `true`.
/// which we can do by the builder method [`CalendarBuilder::treat_weekend_as_bizday`] or
/// the deserializer of the JSON format.
///
/// # Combination of Calendars
/// Calendar can be combined with other calendars in two manners
/// - any-closed strategy: a day is a holiday if it is a holiday in any of the given calendars
/// - all-closed strategy: a day is a holiday if it is a holiday day in all of the given calendars
///
/// These are implemented by [`Calendar::of_any_closed`] and [`Calendar::of_all_closed`] respectively.
///
/// From other point of view, when we focus on the set of non-business days,
/// the any-closed strategy is equivalent to the union of the sets of non-business days
/// and the all-closed strategy is equivalent to the intersection of the sets of non-business days.
/// Hence, these are implemented by the [`BitOr`] and [`BitAnd`] operators respectively.
///
/// ```
/// use qrs_chrono::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
///
/// let cal1 = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .build()
///     .unwrap();
///
/// let cal2 = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 5)])
///     .with_extra_business_days(vec![])
///     .build()
///     .unwrap();
///
/// let cal = cal1 | cal2;
/// assert!(cal.is_holiday(&ymd(2021, 1, 1)).unwrap());
/// assert!(cal.is_holiday(&ymd(2021, 1, 5)).unwrap());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Calendar(Arc<_CalendarData>);

//
// display, serde
//
#[cfg(feature = "serde")]
impl serde::Serialize for Calendar {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
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

#[cfg(feature = "serde")]
impl schemars::JsonSchema for Calendar {
    fn schema_name() -> String {
        "Calendar".to_string()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_chrono::Calendar".into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <_CalendarData as schemars::JsonSchema>::json_schema(gen)
    }
}

//
// construction
//
impl Default for Calendar {
    #[inline]
    fn default() -> Self {
        Self::_new(
            Vec::new(),
            Vec::new(),
            NaiveDate::MIN,
            NaiveDate::MAX,
            false,
        )
        .expect("Default calendar must be valid")
    }
}

impl Calendar {
    fn _new(
        extra_holds: Vec<NaiveDate>,
        extra_bizds: Vec<NaiveDate>,
        valid_from: NaiveDate,
        valid_to: NaiveDate,
        treat_weekend_as_business_day: bool,
    ) -> anyhow::Result<Self> {
        _CalendarData::new(
            extra_holds,
            extra_bizds,
            valid_from,
            valid_to,
            treat_weekend_as_business_day,
        )
        .map(Arc::new)
        .map(Self)
    }

    /// Create a new calendar builder [CalendarBuilder] with default values.
    pub fn builder() -> CalendarBuilder {
        CalendarBuilder::new()
    }

    /// Create a new calendar from multiple caneldars with any-closed strategy.
    /// With this strategy, a day is a holiday if it is a holiday in any of the given calendars.
    ///
    /// For example, if today is not a holiday of Tokyo but a holiday of New York,
    /// the day is considered as a holiday in the new calendar.
    #[inline]
    pub fn of_any_closed<'a, It>(cals: It) -> Self
    where
        It: IntoIterator<Item = &'a Self>,
    {
        let cals = cals.into_iter().map(|c| anyhow::Ok(Cow::Borrowed(c)));
        Self::any_closed_try_from(cals).unwrap()
    }

    pub fn any_closed_try_from<'a, It, E>(cals: It) -> Result<Self, E>
    where
        It: IntoIterator<Item = Result<Cow<'a, Self>, E>>,
    {
        let mut extra_holds = HashSet::new();
        let mut extra_bizds: Option<HashSet<_>> = None;
        let mut valid_from = NaiveDate::MIN;
        let mut valid_to = NaiveDate::MAX;
        let mut treat_weekend_as_business_day = None;

        for cal in cals {
            let cal = &cal?.0;
            if let Some(ref mut flag) = treat_weekend_as_business_day {
                *flag &= cal.treat_weekend_as_business_day;
            } else {
                treat_weekend_as_business_day = Some(cal.treat_weekend_as_business_day);
            }
            extra_holds.extend(cal.extra_holds.iter().copied());

            if !treat_weekend_as_business_day.unwrap() {
                match extra_bizds {
                    None => extra_bizds = Some(cal.extra_bizds.iter().copied().collect()),
                    Some(ref mut bizds) => {
                        bizds.retain(|d| cal.extra_bizds.contains(d));
                    }
                }
            }
            valid_from = valid_from.max(cal.valid_from);
            valid_to = valid_to.min(cal.valid_to);
        }
        let res = Self::_new(
            extra_holds.into_iter().collect(),
            if treat_weekend_as_business_day.unwrap_or_default() {
                // if weekends are business days, extra business days must be empty
                Vec::new()
            } else {
                extra_bizds.into_iter().flatten().collect()
            },
            valid_from,
            valid_to,
            treat_weekend_as_business_day.unwrap_or_default(),
        )
        .expect("AnyClosed of valid calendars must be valid");
        Ok(res)
    }

    /// Create a new calendar from multiple caneldars with all-closed strategy.
    /// With this strategy, a day is a holiday if it is a holiday day in all of the given calendars.
    ///
    /// For example, if today is a holiday of Tokyo but not a holiday of New York,
    /// the day is considered as a business day in the new calendar.
    #[inline]
    pub fn all_closed_from<'a, It>(cals: It) -> Self
    where
        It: IntoIterator<Item = &'a Self>,
    {
        let cals = cals.into_iter().map(|c| anyhow::Ok(Cow::Borrowed(c)));
        Self::all_closed_try_from(cals).unwrap()
    }

    pub fn all_closed_try_from<'a, It, E>(cals: It) -> Result<Self, E>
    where
        It: IntoIterator<Item = Result<Cow<'a, Self>, E>>,
    {
        let mut extra_holds: Option<HashSet<_>> = None;
        let mut extra_bizds = HashSet::new();
        let mut valid_from = NaiveDate::MIN;
        let mut valid_to = NaiveDate::MAX;
        let mut treat_weekend_as_business_day = None;

        for cal in cals {
            let cal = &cal?.0;
            if let Some(ref mut flag) = treat_weekend_as_business_day {
                *flag |= cal.treat_weekend_as_business_day;
            } else {
                treat_weekend_as_business_day = Some(cal.treat_weekend_as_business_day);
            }
            if !treat_weekend_as_business_day.unwrap() {
                extra_bizds.extend(cal.extra_bizds.iter().copied());
            }

            match extra_holds {
                None => extra_holds = Some(cal.extra_holds.iter().copied().collect()),
                Some(ref mut holds) => {
                    holds.retain(|d| cal.extra_holds.contains(d));
                }
            }
            valid_from = valid_from.max(cal.valid_from);
            valid_to = valid_to.min(cal.valid_to);
        }
        let res = Self::_new(
            extra_holds.into_iter().flatten().collect(),
            if treat_weekend_as_business_day.unwrap_or_default() {
                // if weekends are business days, extra business days must be empty
                Vec::new()
            } else {
                extra_bizds.into_iter().collect()
            },
            valid_from,
            valid_to,
            treat_weekend_as_business_day.unwrap_or_default(),
        )
        .expect("AllClosed of valid calendars must be valid");
        Ok(res)
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
    /// The valid period is a half-open interval `valid_from..valid_to`
    /// where `valid_from <= valid_to` always holds.
    #[inline]
    pub fn valid_period(&self) -> Range<NaiveDate> {
        self.0.valid_from..self.0.valid_to
    }

    /// Check if the given date is valid in the calendar.
    ///
    /// Because this object cannot have infinitely many holidays and business days,
    /// some days are not supported by this calendar.
    /// This method check that the given date is supported by the calendar.
    ///
    /// This is equivalent to `self.valid_period().contains(date)`.
    #[inline]
    pub fn is_supported(&self, date: &NaiveDate) -> bool {
        self.valid_period().contains(date)
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

    /// Flag for treatment of weekend.
    ///
    /// If this flag is `true`, weekends are treated as business days.
    /// Otherwise, weekends are treated as holidays.
    #[inline]
    pub fn treat_weekend_as_bizday(&self) -> bool {
        self.0.treat_weekend_as_business_day
    }

    /// Count the business days between the given dates.
    pub fn num_bizdays(&self, start: NaiveDate, end: NaiveDate) -> usize {
        if end <= start {
            return 0;
        }
        if self.treat_weekend_as_bizday() {
            let hol_stt = self.extra_holidays().partition_point(|d| *d < start);
            let hol_end = self.extra_holidays().partition_point(|d| *d < end);
            return ((end - start).num_days() - (hol_end - hol_stt) as i64) as usize;
        }
        let prev_stt_mon = start - chrono::Days::new(start.weekday().num_days_from_monday() as _);
        let prev_end_mon = end - chrono::Days::new(end.weekday().num_days_from_monday() as _);
        let naive_count = (prev_end_mon - prev_stt_mon).num_days() / 7 * 5
            - start.weekday().num_days_from_monday().min(5) as i64
            + end.weekday().num_days_from_monday().min(5) as i64;

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

        (naive_count - extra_hols + extra_bds) as usize
    }

    /// Check if the given date is a holiday.
    ///
    /// If the given date is not supported by the calendar, this method returns [`None`].
    #[inline]
    pub fn is_holiday(&self, date: &NaiveDate) -> Option<bool> {
        if !self.is_supported(date) {
            return None;
        }
        let is_default_hold = !self.treat_weekend_as_bizday()  // weekends are holiday
            && 5 < date.weekday().number_from_monday(); // and date is weekend
        if is_default_hold {
            Some(self.0.extra_bizds.binary_search(date).is_err())
        } else {
            Some(self.0.extra_holds.binary_search(date).is_ok())
        }
    }

    /// Check if the given date is a business day.
    ///
    /// If the given date is not supported by the calendar, this method returns [`None`].
    #[inline]
    pub fn is_bizday(&self, date: &NaiveDate) -> Option<bool> {
        if !self.is_supported(date) {
            return None;
        }
        let is_default_hold = !self.treat_weekend_as_bizday()  // weekends are holiday
            && 5 < date.weekday().number_from_monday(); // and date is weekend
        if is_default_hold {
            Some(self.0.extra_bizds.binary_search(date).is_ok())
        } else {
            Some(self.0.extra_holds.binary_search(date).is_err())
        }
    }

    /// Iterator over the business days from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::Calendar;
    /// use chrono::NaiveDate;
    ///
    /// let cal = Calendar::builder()
    ///     .with_valid_period(
    ///         NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
    ///         NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()
    ///     )
    ///     .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
    ///     .with_extra_business_days(vec![])
    ///     .build()
    ///     .unwrap();
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
    pub fn iter_bizdays(
        &self,
        start: NaiveDate,
    ) -> impl DoubleEndedIterator<Item = NaiveDate> + '_ {
        DateIterator {
            cur: start,
            from: self.0.valid_from,
            to: self.0.valid_to,
        }
        .filter(move |d| self.is_bizday(d).unwrap_or(false))
    }

    /// Iterator over the holidays from the given date.
    ///
    /// This iterator ends when iterated date is out of the valid period of the calendar.
    ///
    /// # Example
    /// ```
    /// use qrs_chrono::Calendar;
    /// use chrono::NaiveDate;
    ///
    /// let cal = Calendar::builder()
    ///     .with_valid_period(
    ///         NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
    ///         NaiveDate::from_ymd_opt(2021, 1, 10).unwrap()
    ///     )
    ///     .with_extra_holidays(vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()])
    ///     .with_extra_business_days(vec![])
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut iter = cal.iter_holidays(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 2).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 3).unwrap()));
    /// assert_eq!(iter.next(), Some(NaiveDate::from_ymd_opt(2021, 1, 9).unwrap()));
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
        .filter(move |d| self.is_holiday(d).unwrap_or(false))
    }
}

//
// operators
//
/// Create
impl BitAnd for Calendar {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::all_closed_from([self, rhs].iter())
    }
}

impl BitAnd<Calendar> for &Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: Calendar) -> Self::Output {
        Calendar::all_closed_from([self, &rhs])
    }
}

impl BitAnd for &Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: Self) -> Self::Output {
        Calendar::all_closed_from([self, rhs])
    }
}

impl BitAnd<&Calendar> for Calendar {
    type Output = Calendar;

    fn bitand(self, rhs: &Self) -> Self::Output {
        Calendar::all_closed_from([&self, rhs])
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
/// Builder of a calendar
///
/// The [`Calendar`] consists of the three data, extra holidays, extra business days, and valid period.
/// (See the documentation of [`Calendar`] for more details)
///
/// This builder provides methods to set these data and build a new calendar.
/// As default, the Saturday and Sunday are considered as holidays.
/// If you want to treat the weekend as business day, set the flag `treat_weekend_as_business_day` to `true`
/// via the method [`CalendarBuilder::treat_weekend_as_bizday`].
///
/// # Example
/// ```
/// use qrs_chrono::Calendar;
///
/// let ymd = |y: i32, m: u32, d: u32| chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
///
/// let cal = Calendar::builder()
///     .with_valid_period(ymd(2021, 1, 1), ymd(2021, 1, 10))
///     .with_extra_holidays(vec![ymd(2021, 1, 1)])
///     .with_extra_business_days(vec![])
///     .treat_weekend_as_bizday(true)  // weekends are business day
///     .build();
///
/// ````
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CalendarBuilder<H = (), B = (), V = ()> {
    extra_holds: H,
    extra_bizds: B,
    valid_from: V,
    valid_to: V,
    treat_weekend_as_business_day: bool,
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
            treat_weekend_as_business_day: false,
        }
    }
}

impl CalendarBuilder {
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
            treat_weekend_as_business_day: self.treat_weekend_as_business_day,
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
            treat_weekend_as_business_day: self.treat_weekend_as_business_day,
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
            treat_weekend_as_business_day: self.treat_weekend_as_business_day,
        }
    }
}

impl<B, H, D> CalendarBuilder<B, H, D> {
    /// Set the flag to treat weekend as business day
    pub fn treat_weekend_as_bizday(
        self,
        treat_weekend_as_business_day: bool,
    ) -> CalendarBuilder<B, H, D> {
        CalendarBuilder {
            extra_holds: self.extra_holds,
            extra_bizds: self.extra_bizds,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            treat_weekend_as_business_day,
        }
    }
}

impl CalendarBuilder<Vec<NaiveDate>, Vec<NaiveDate>, NaiveDate> {
    /// Build a new calendar.
    ///
    /// # Errors
    /// - If the given extra holidays are not weekdays (when weekends are treated as holidays)
    /// - If the given extra business days are not weekends
    pub fn build(self) -> anyhow::Result<Calendar> {
        Calendar::_new(
            self.extra_holds,
            self.extra_bizds,
            self.valid_from,
            self.valid_to,
            self.treat_weekend_as_business_day,
        )
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_new() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        );
        assert!(cal.is_ok());

        // duplicated extra holidays, unsorted extra holidays are allowed
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        );
        assert!(cal.is_ok());

        // invalid extra holidays
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        );
        assert!(cal.is_err());

        // when treat_weekend_as_business_day is true, weekends are allowed as extra holidays
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        );
        assert!(cal.is_ok());

        // invalid extra business days
        let cal = Calendar::_new(
            vec![],
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        );
        assert!(cal.is_err());

        // invalid valid period
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 10),
            ymd(2021, 1, 1),
            false,
        );
        assert!(cal.is_err());

        // extra holidays and business days are not allowed when treat_weekend_as_business_day is true
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        );
        assert!(cal.is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let json = serde_json::to_string(&cal).unwrap();
        assert_eq!(
            json,
            r#"{"extra_holidays":["2021-01-01"],"extra_business_days":["2021-01-02"],"valid_from":"2021-01-01","valid_to":"2021-01-10","treat_weekend_as_business_day":false}"#
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deserialize() {
        let json = r#"{"extra_holidays":["2021-01-01"],"extra_business_days":["2021-01-02"],"valid_from":"2021-01-01","valid_to":"2021-01-10"}"#;
        let cal: Calendar = serde_json::from_str(json).unwrap();
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2)]);
        assert!(!cal.treat_weekend_as_bizday());
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
        let json = r#"{"extra_holidays":["2021-01-01"],"extra_business_days":[],"valid_from":"2021-01-01","valid_to":"2021-01-10","treat_weekend_as_business_day":true}"#;
        let cal: Calendar = serde_json::from_str(json).unwrap();
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(cal.treat_weekend_as_bizday());
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
    }

    #[test]
    fn test_of_any_closed() {
        // empty
        let cal = Calendar::of_any_closed(vec![]);
        assert_eq!(cal.extra_holidays(), &[]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(!cal.treat_weekend_as_bizday());

        // single
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal = Calendar::of_any_closed(vec![&cal1]);
        assert_eq!(cal, cal1);

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        let cal = Calendar::of_any_closed(vec![&cal1]);
        assert_eq!(cal, cal1);

        // multiple
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2), ymd(2021, 1, 3)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        let cal = Calendar::of_any_closed(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2),]);
        assert!(!cal.treat_weekend_as_bizday());

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        let cal = Calendar::of_any_closed(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2),]);
        assert!(!cal.treat_weekend_as_bizday());

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();

        let cal = Calendar::of_any_closed(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(cal.treat_weekend_as_bizday());
    }

    #[test]
    fn test_of_all_closed() {
        // empty
        let cal = Calendar::all_closed_from(vec![]);
        assert_eq!(cal.extra_holidays(), &[]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(!cal.treat_weekend_as_bizday());

        // single
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal = Calendar::all_closed_from(vec![&cal1]);
        assert_eq!(cal, cal1);

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        let cal = Calendar::all_closed_from(vec![&cal1]);
        assert_eq!(cal, cal1);

        // multiple
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 3)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        let cal = Calendar::all_closed_from(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2), ymd(2021, 1, 3)]);
        assert!(!cal.treat_weekend_as_bizday());

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        let cal = Calendar::all_closed_from(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[]);
        assert!(cal.treat_weekend_as_bizday());

        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 3)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        let cal = Calendar::all_closed_from(vec![&cal1, &cal2]);
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2), ymd(2021, 1, 3)]);
        assert!(!cal.treat_weekend_as_bizday());
    }

    #[test]
    fn test_valid_period() {
        let cal = Calendar::_new(vec![], vec![], ymd(2021, 1, 1), ymd(2021, 1, 10), false).unwrap();
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );
    }

    #[test]
    fn test_is_supported() {
        let cal = Calendar::_new(vec![], vec![], ymd(2021, 1, 1), ymd(2021, 1, 10), false).unwrap();
        assert!(!cal.is_supported(&ymd(2020, 12, 31)));
        assert!(cal.is_supported(&ymd(2021, 1, 1)));
        assert!(cal.is_supported(&ymd(2021, 1, 9)));
        assert!(!cal.is_supported(&ymd(2021, 1, 10)));
        assert!(!cal.is_supported(&ymd(2021, 1, 11)));
    }

    #[test]
    fn test_is_holiday() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        assert_eq!(cal.is_holiday(&ymd(2020, 12, 30)), None);
        assert_eq!(cal.is_holiday(&ymd(2020, 12, 31)), None);
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 1)), Some(true));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 2)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 3)), Some(true));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 4)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 5)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 6)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 7)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 8)), Some(false));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 9)), Some(true));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 10)), None);

        // when treat_weekend_as_business_day is true, weekends are allowed as extra holidays
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 1)), Some(true));
        assert_eq!(cal.is_holiday(&ymd(2021, 1, 2)), Some(true));
        assert_eq!(
            cal.is_holiday(&ymd(2021, 1, 3)),
            Some(false) // Sunday
        );
    }

    #[test]
    fn test_is_business_day() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();

        assert_eq!(cal.is_bizday(&ymd(2021, 1, 1)), Some(false));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 2)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 3)), Some(false));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 4)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 5)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 6)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 7)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 8)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 9)), Some(false));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 10)), None);

        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            true,
        )
        .unwrap();

        assert_eq!(cal.is_bizday(&ymd(2021, 1, 1)), Some(false));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 2)), Some(false));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 3)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 4)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 5)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 6)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 7)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 8)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 9)), Some(true));
        assert_eq!(cal.is_bizday(&ymd(2021, 1, 10)), None);
    }

    #[test]
    fn test_iter_bizdays() {
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 10),
            false,
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

        // reverse
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
            false,
        )
        .unwrap();

        let mut iter = cal.iter_holidays(ymd(2021, 1, 1));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
        assert_eq!(iter.next(), None);

        // reverse
        let mut iter = cal.iter_holidays(ymd(2021, 1, 9)).rev();
        assert_eq!(iter.next(), Some(ymd(2021, 1, 9)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 3)));
        assert_eq!(iter.next(), Some(ymd(2021, 1, 1)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_num_bizdays() {
        let from = ymd(2021, 1, 1);
        let to = ymd(2021, 12, 30);

        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2022, 1, 10),
            false,
        )
        .unwrap();

        let exp = from
            .iter_days()
            .take_while(|d| d < &to)
            .filter(|d| cal.is_bizday(d) == Some(true))
            .count();

        let num = cal.num_bizdays(from, to);
        assert_eq!(num, exp);

        let num = cal.num_bizdays(to, from);
        assert_eq!(num, 0);

        // weekends are treated as business day
        let cal = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 2)],
            vec![],
            ymd(2021, 1, 1),
            ymd(2022, 1, 10),
            true,
        )
        .unwrap();

        let exp = from
            .iter_days()
            .take_while(|d| d < &to)
            .filter(|d| cal.is_bizday(d) == Some(true))
            .count();
        assert_eq!(cal.num_bizdays(from, to), exp);

        let num = cal.num_bizdays(to, from);
        assert_eq!(num, 0);
    }

    #[test]
    fn test_bitor() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1)],
            vec![ymd(2021, 1, 2), ymd(2021, 1, 3)],
            ymd(2020, 12, 31),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 15),
            false,
        )
        .unwrap();

        let cal = &cal1 | &cal2;
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 1), ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2),]);
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );

        assert_eq!(cal, &cal1 | cal2.clone());
        assert_eq!(cal, cal1.clone() | &cal2);
        assert_eq!(cal, cal1 | cal2);
    }

    #[test]
    fn test_bitand() {
        let cal1 = Calendar::_new(
            vec![ymd(2021, 1, 1), ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 3)],
            ymd(2020, 12, 31),
            ymd(2021, 1, 10),
            false,
        )
        .unwrap();
        let cal2 = Calendar::_new(
            vec![ymd(2021, 1, 5)],
            vec![ymd(2021, 1, 2)],
            ymd(2021, 1, 1),
            ymd(2021, 1, 15),
            false,
        )
        .unwrap();

        let cal = &cal1 & &cal2;
        assert_eq!(cal.extra_holidays(), &[ymd(2021, 1, 5)]);
        assert_eq!(cal.extra_bizdays(), &[ymd(2021, 1, 2), ymd(2021, 1, 3)]);
        assert_eq!(
            cal.valid_period(),
            Range {
                start: ymd(2021, 1, 1),
                end: ymd(2021, 1, 10)
            }
        );

        assert_eq!(cal, &cal1 & cal2.clone());
        assert_eq!(cal, cal1.clone() & &cal2);
        assert_eq!(cal, cal1 & cal2);
    }
}
