use qchrono::{ext::chrono::Datelike, timepoint::Date};

// -----------------------------------------------------------------------------
// YearFrac
// -----------------------------------------------------------------------------
pub trait YearFrac<D: Datelike = Date> {
    type Error;

    fn year_frac(&self, start: &D, end: &D) -> Result<f64, Self::Error>;
}
