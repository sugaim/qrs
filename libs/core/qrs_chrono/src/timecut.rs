use chrono::NaiveDate;

use crate::DateTime;

// -----------------------------------------------------------------------------
// TimeCut
//
/// A trait for converting a date to a datetime.
///
/// For example, an object of this trait can be work as a timecut, such as NYK close.
pub trait TimeCut {
    type Err;
    type Tz: chrono::TimeZone;

    fn to_datetime(&self, date: NaiveDate) -> Result<DateTime<Self::Tz>, Self::Err>;
}
