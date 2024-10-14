use qchrono::{ext::chrono::Datelike, timepoint::Date};

// -----------------------------------------------------------------------------
// YearFrac
// StateLessYearFrac
// -----------------------------------------------------------------------------
/// Year fraction
pub trait YearFrac<D: Datelike = Date> {
    type Error;

    /// Calculate a year fraction between two dates.
    fn year_frac(&self, start: &D, end: &D) -> Result<f64, Self::Error>;
}

/// Tag for a stateless year fraction.
///
/// This trait is used to mark a year fraction as stateless.
pub trait StateLessYearFrac<D: Datelike = Date>: YearFrac<D> + Default {}
