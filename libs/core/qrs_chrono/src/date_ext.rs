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
}

impl<T: Datelike> DateExtensions for T {}
