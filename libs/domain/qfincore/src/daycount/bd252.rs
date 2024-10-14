use std::sync::Arc;

use qchrono::{
    calendar::{Calendar, CalendarError, CalendarSym},
    timepoint::Date,
};

use super::YearFrac;

// -----------------------------------------------------------------------------
// Bd252
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct _Data {
    sym: CalendarSym,
    cal: Calendar,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bd252 {
    data: Arc<_Data>,
}

impl Bd252 {
    #[inline]
    pub fn new(sym: CalendarSym, cal: Calendar) -> Self {
        Self {
            data: Arc::new(_Data { sym, cal }),
        }
    }

    #[inline]
    pub fn calendar(&self) -> &Calendar {
        &self.data.cal
    }
    #[inline]
    pub fn calendar_sym(&self) -> &CalendarSym {
        &self.data.sym
    }
}

impl YearFrac for Bd252 {
    type Error = CalendarError;

    #[inline]
    fn year_frac(&self, start: &Date, end: &Date) -> Result<f64, Self::Error> {
        if end < start {
            return self.year_frac(end, start).map(std::ops::Neg::neg);
        }
        let days = self.data.cal.num_bizdays(start..end)?;
        Ok(days as f64 / 252.0)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use qchrono::timepoint::Weekday;
    use rstest::rstest;

    fn ymd(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    fn instance() -> Bd252 {
        let sym = CalendarSym::from_str("TKY").unwrap();
        let cal = Calendar::builder()
            .with_valid_period(ymd(2000, 1, 1), ymd(2999, 12, 31))
            .with_extra_business_days(vec![])
            .with_extra_holidays(vec![ymd(2021, 1, 13)])
            .with_holiday_weekdays(vec![Weekday::Sun, Weekday::Sat])
            .build()
            .unwrap();
        Bd252 {
            data: Arc::new(_Data { sym, cal }),
        }
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 2), 1. / 252.)]
    #[case(ymd(2021, 1, 2), ymd(2021, 1, 3), 0. / 252.)]
    #[case(ymd(2021, 1, 4), ymd(2021, 1, 11), 5. / 252.)]
    #[case(ymd(2021, 1, 11), ymd(2021, 1, 18), 4. / 252.)]
    #[case(ymd(2021, 1, 1), ymd(2021, 2, 1), 20. / 252.)]
    #[case(ymd(2021, 3, 1), ymd(2022, 3, 1), 261. / 252.)]
    fn test_year_fraction(#[case] start: Date, #[case] end: Date, #[case] expected: f64) {
        let bd252 = instance();
        let dcf = bd252.year_frac(&start, &end).unwrap();
        let rev = bd252.year_frac(&end, &start).unwrap();

        approx::assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(dcf, -rev, epsilon = 1e-10);
    }
}
