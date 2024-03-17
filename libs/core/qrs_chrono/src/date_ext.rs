use chrono::Datelike;

// -----------------------------------------------------------------------------
// DateExtensions
//
pub trait DateExtensions: Datelike {
    #[inline]
    fn is_leap_year(&self) -> bool {
        let y = self.year();
        y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
    }

    #[inline]
    fn is_leap_day(&self) -> bool {
        self.is_leap_year() && self.month() == 2 && self.day() == 29
    }
}

impl<T: Datelike> DateExtensions for T {}

// =============================================================================
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(1900, false)]
    #[case(1970, false)]
    #[case(1972, true)]
    #[case(2000, true)]
    #[case(2001, false)]
    #[case(2004, true)]
    #[case(2020, true)]
    #[case(2100, false)]
    #[case(2400, true)]
    fn test_is_leap_year(#[case] y: i32, #[case] expected: bool) {
        let date = chrono::NaiveDate::from_ymd_opt(y, 1, 1).unwrap();

        let is_leap = date.is_leap_year();

        assert_eq!(is_leap, expected);
    }

    #[rstest]
    #[case(1900, 2, 28, false)]
    #[case(1970, 2, 28, false)]
    #[case(1972, 2, 28, false)]
    #[case(1972, 2, 29, true)]
    #[case(2000, 2, 29, true)]
    #[case(2001, 2, 28, false)]
    #[case(2004, 2, 28, false)]
    #[case(2004, 2, 29, true)]
    #[case(2020, 2, 28, false)]
    #[case(2020, 2, 29, true)]
    #[case(2100, 2, 28, false)]
    #[case(2400, 2, 28, false)]
    #[case(2400, 2, 29, true)]
    fn test_is_leap_day(#[case] y: i32, #[case] m: u32, #[case] d: u32, #[case] expected: bool) {
        let date = chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();

        let is_leap = date.is_leap_day();

        assert_eq!(is_leap, expected);
    }
}
