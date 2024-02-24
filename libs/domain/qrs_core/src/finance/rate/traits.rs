use crate::{
    chrono::GenericDateTime,
    finance::daycount::DayCount,
    num::{Real, Scalar},
};

// -----------------------------------------------------------------------------
// Rate
//
pub trait Rate: Sized {
    type Value: Real;
    type Convention: DayCount;

    /// Get day count convention which this rate obeys.
    fn convention(&self) -> Self::Convention;

    /// Value of annual rate. Unit is 1. Not percent nor bps.
    fn value(&self) -> Self::Value;

    /// Calculate change ratio between two dates.
    fn ratio_between<Tz>(&self, from: &GenericDateTime<Tz>, to: &GenericDateTime<Tz>) -> Self::Value
    where
        Tz: chrono::TimeZone,
    {
        let dcf = self.convention().dcf(from, to);
        let dcf = <Self::Value as Scalar>::nearest_value_of(dcf);
        self.value() * &dcf
    }

    /// Value of annual rate. Unit is percent.
    fn percent(&self) -> Self::Value {
        const MULT: f64 = 1e2;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.value() * &mult
    }

    /// Value of annual rate. Unit is bps.
    fn bps(&self) -> Self::Value {
        const MULT: f64 = 1e4;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.value() * &mult
    }
}
