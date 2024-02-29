use chrono::{NaiveDate, NaiveTime, TimeZone};

// -----------------------------------------------------------------------------
// DateTimeBuilder
//

/// A builder for creating a `qrs_core::chrono::DateTime`.
///
/// # Example
/// ```
/// use qrs_core::chrono::DateTimeBuilder;
///
/// let datetime = DateTimeBuilder::new()
///     .with_ymd(2021, 1, 1).unwrap()
///     .with_hms(10, 42, 11).unwrap()
///     .with_fixed_offset(9 * 3600).unwrap()
///     .build();
///
/// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTimeBuilder<D = (), T = (), Tz = ()> {
    date: D,
    time: T,
    timezone: Tz,
}

pub type DateToDateTime<Tz> = DateTimeBuilder<NaiveDate, (), Tz>;

//
// construction
//
impl Default for DateTimeBuilder<(), (), ()> {
    #[inline]
    fn default() -> Self {
        Self {
            date: (),
            time: (),
            timezone: (),
        }
    }
}

impl DateTimeBuilder<(), (), ()> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

//
// setters
//
impl<D, Tz> DateTimeBuilder<D, (), Tz> {
    /// Set time to the builder.
    /// Available types are implementations of [chrono::Timelike].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_time(&chrono::NaiveTime::from_hms_opt(10, 42, 11).unwrap())
    ///     .with_fixed_offset(9 * 3600).unwrap()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_time<T>(self, time: &T) -> DateTimeBuilder<D, chrono::NaiveTime, Tz>
    where
        T: chrono::Timelike,
    {
        DateTimeBuilder {
            date: self.date,
            time: NaiveTime::from_hms_opt(time.hour(), time.minute(), time.second())
                .expect("Argument 'time' is expected to be a valid time"),
            timezone: self.timezone,
        }
    }

    /// Set time to the builder.
    /// For invalid time, including non-normalized time, this method returns [None].
    /// Details for invalid time is described in [chrono::NaiveTime::from_hms_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_fixed_offset(9 * 3600).unwrap()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    ///
    /// let invalid = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 60);
    ///
    /// assert!(invalid.is_none());
    /// ```
    #[inline]
    pub fn with_hms(
        self,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Option<DateTimeBuilder<D, chrono::NaiveTime, Tz>> {
        NaiveTime::from_hms_opt(hour, minute, second).map(|time| DateTimeBuilder {
            date: self.date,
            time,
            timezone: self.timezone,
        })
    }

    /// Set time to the builder.
    /// For invalid time, including non-normalized time, this method returns [None].
    /// Details for invalid time is described in [chrono::NaiveTime::from_hms_milli_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms_millli(10, 42, 11, 123).unwrap()
    ///     .with_fixed_offset(9 * 3600).unwrap()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11.123+09:00");
    ///
    /// let invalid = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms_millli(10, 42, 60, 123);
    ///
    /// assert!(invalid.is_none());
    /// ```
    #[inline]
    pub fn with_hms_millli(
        self,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: u32,
    ) -> Option<DateTimeBuilder<D, chrono::NaiveTime, Tz>> {
        NaiveTime::from_hms_milli_opt(hour, minute, second, millisecond).map(|time| {
            DateTimeBuilder {
                date: self.date,
                time,
                timezone: self.timezone,
            }
        })
    }
}
impl<T, Tz> DateTimeBuilder<(), T, Tz> {
    /// Set date to the builder.
    /// Available types are implementations of [chrono::Datelike].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_date(&chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap())
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_utc()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+00:00");
    /// ```
    #[inline]
    pub fn with_date<D>(self, date: &D) -> DateTimeBuilder<chrono::NaiveDate, T, Tz>
    where
        D: chrono::Datelike,
    {
        DateTimeBuilder {
            date: NaiveDate::from_ymd_opt(date.year(), date.month(), date.day())
                .expect("Argument 'date' is expected to be a valid date"),
            time: self.time,
            timezone: self.timezone,
        }
    }

    /// Set date to the builder.
    /// For invalid date, including non-normalized date, this method returns [None].
    /// Details for invalid date is described in [chrono::NaiveDate::from_ymd_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_fixed_offset(9 * 3600).unwrap()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    ///
    /// let invalid = DateTimeBuilder::new()
    ///     .with_ymd(2021, 2, 29);
    ///
    /// assert!(invalid.is_none());
    /// ```
    #[inline]
    pub fn with_ymd(
        self,
        year: i32,
        month: u32,
        day: u32,
    ) -> Option<DateTimeBuilder<chrono::NaiveDate, T, Tz>> {
        NaiveDate::from_ymd_opt(year, month, day).map(|date| DateTimeBuilder {
            date,
            time: self.time,
            timezone: self.timezone,
        })
    }
}
impl<D, T> DateTimeBuilder<D, T, ()> {
    /// Set timezone to the builder.
    /// Available types are implementations of [chrono::TimeZone].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_timezone(chrono::FixedOffset::east_opt(9 * 3600).unwrap())
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn with_timezone<Tz: TimeZone>(self, timezone: Tz) -> DateTimeBuilder<D, T, Tz> {
        DateTimeBuilder {
            date: self.date,
            time: self.time,
            timezone,
        }
    }

    /// Set fixed offset timezone to the builder.
    /// We use east offset rather than west one as argument because the offset is positive.
    ///
    /// For invalid offset, this method returns [None].
    /// Details for invalid offset is described in [chrono::FixedOffset::east_opt].
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_fixed_offset(9 * 3600).unwrap()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    ///
    /// let invalid = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_fixed_offset(24 * 3600);
    ///
    /// assert!(invalid.is_none());
    /// ```
    #[inline]
    pub fn with_fixed_offset(
        self,
        secs: i32,
    ) -> Option<DateTimeBuilder<D, T, chrono::FixedOffset>> {
        chrono::FixedOffset::east_opt(secs).map(|timezone| DateTimeBuilder {
            date: self.date,
            time: self.time,
            timezone,
        })
    }

    /// Set UTC timezone to the builder.
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_utc()
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+00:00");
    /// ```
    #[inline]
    pub fn with_utc(self) -> DateTimeBuilder<D, T, chrono::Utc> {
        self.with_timezone(chrono::Utc)
    }
}

//
// build
//
impl<Tz: TimeZone> DateTimeBuilder<NaiveDate, NaiveTime, Tz> {
    /// Build a `DateTime` from the builder with stored date, time and timezone.
    /// This methos is available only after setting date, time and timezone.
    ///
    /// # Example
    /// ```
    /// use qrs_core::chrono::DateTimeBuilder;
    ///
    /// let datetime = DateTimeBuilder::new()
    ///     .with_ymd(2021, 1, 1).unwrap()
    ///     .with_hms(10, 42, 11).unwrap()
    ///     .with_timezone(chrono::FixedOffset::east_opt(9 * 3600).unwrap())
    ///     .build();
    ///
    /// assert_eq!(datetime.to_rfc3339(), "2021-01-01T10:42:11+09:00");
    /// ```
    #[inline]
    pub fn build(self) -> super::datetime::GenericDateTime<Tz> {
        self.timezone
            .from_local_datetime(&self.date.and_time(self.time))
            .single()
            .expect("The date and time is expected to be valid")
            .into()
    }
}
