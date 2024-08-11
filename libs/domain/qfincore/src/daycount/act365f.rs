use std::convert::Infallible;

use qchrono::timepoint::{Date, DateTime};

use super::YearFrac;

// -----------------------------------------------------------------------------
// Act365f
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Default)]
pub struct Act365f;

//
// behavior
//
impl YearFrac for Act365f {
    type Error = Infallible;

    #[inline]
    fn year_frac(&self, start: &Date, end: &Date) -> Result<f64, Self::Error> {
        let days = (*end - *start).num_days() as f64;
        Ok(days / 365.0)
    }
}

impl YearFrac<DateTime> for Act365f {
    type Error = Infallible;

    #[inline]
    fn year_frac(&self, start: &DateTime, end: &DateTime) -> Result<f64, Self::Error> {
        let days = (end - start).approx_secs();
        Ok(days / (365.0 * 24.0 * 60.0 * 60.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn ymd(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn test() {}

    #[rstest]
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 2), 1. / 365.)]
    #[case(ymd(2021, 1, 1), ymd(2021, 2, 1), 31. / 365.)]
    #[case(ymd(2021, 1, 1), ymd(2022, 1, 1), 1.)]
    #[case(ymd(2024, 1, 1), ymd(2025, 1, 1), 366. / 365.)]
    #[case(ymd(2021, 7, 13), ymd(2021, 7, 25), 12. / 365.)]
    fn test_year_fraction(#[case] start: Date, #[case] end: Date, #[case] expected: f64) {
        let dcf = Act365f.year_frac(&start, &end).unwrap();
        let rev = Act365f.year_frac(&end, &start).unwrap();

        approx::assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(dcf, -rev, epsilon = 1e-10);
    }

    #[rstest]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 1. / 365.)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-02-01T00:00:00Z".parse().unwrap(), 31. / 365.)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2022-01-01T00:00:00Z".parse().unwrap(), 1.)]
    #[case("2024-01-01T00:00:00Z".parse().unwrap(), "2025-01-01T00:00:00Z".parse().unwrap(), 366. / 365.)]
    #[case("2021-07-13T00:00:00Z".parse().unwrap(), "2021-07-25T00:00:00Z".parse().unwrap(), 12. / 365.)]
    #[case("2021-01-01T09:22:33Z".parse().unwrap(), "2021-01-01T11:31:55Z".parse().unwrap(), (22. + 9. * 60. + 2. * 3600.) / 24. / 60. / 60. / 365.)]
    #[case("2021-01-01T09:22:33+09:00".parse().unwrap(), "2021-01-01T11:31:55+09:00".parse().unwrap(), (22. + 9. * 60. + 2. * 3600.) / 24. / 60. / 60. / 365.)]
    #[case("2021-01-01T09:22:33+09:00".parse().unwrap(), "2021-01-01T11:01:55-05:30".parse().unwrap(), (22. + 9. * 60. + 16. * 3600.) / 24. / 60. / 60. / 365.)]
    fn test_year_fraction_datetime(
        #[case] start: DateTime,
        #[case] end: DateTime,
        #[case] expected: f64,
    ) {
        let dcf = Act365f.year_frac(&start, &end).unwrap();
        let rev = Act365f.year_frac(&end, &start).unwrap();

        approx::assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(dcf, -rev, epsilon = 1e-10);
    }
}
