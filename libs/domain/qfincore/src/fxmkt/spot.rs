use qchrono::{
    calendar::{Calendar, HolidayAdj},
    ext::chrono::offset::LocalResult,
    timepoint::{Date, DateTime},
};

use crate::CcyPair;

// -----------------------------------------------------------------------------
// FxSpotMkt
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct FxSpotMkt {
    pub spot_lag: u8,
    pub settle_cal: Calendar,
}

impl FxSpotMkt {
    /// Return the spot date of the given value date.
    ///
    /// This returns [None] if the spot date is out of supported range.
    #[inline]
    pub fn spot_date_of(&self, value_date: Date) -> anyhow::Result<Date> {
        let err = || {
            anyhow::anyhow!(
            "Fail to calculate spot date for value date({}) because the date is not supported by the calendar",
            value_date
        )
        };
        let d = HolidayAdj::Following
            .adjust(value_date, &self.settle_cal)
            .ok_or_else(err)?;
        self.settle_cal
            .iter_bizdays(d)
            .nth(self.spot_lag as usize)
            .ok_or_else(err)
    }

    /// Return the spot datetime of the given value date.
    ///
    /// This returns [None] if the spot date is out of supported range
    /// or the spot date is not uniquely determined due to timezone issue.
    pub fn spot_datetime_of(&self, value_date: &DateTime) -> anyhow::Result<DateTime> {
        let dt = self
            .spot_date_of(value_date.date())?
            .and_time(value_date.time())
            .and_local_timezone(value_date.timezone())
            .map(Into::into);
        match dt {
            LocalResult::Single(dt) => Ok(dt),
            _ => Err(anyhow::anyhow!(
                "Fail to determine spot datetime for value datetime({}) because the date is not uniquely determined due to timezone issue",
                value_date
            )),
        }
    }
}

// -----------------------------------------------------------------------------
// FxSpotMktSrc
// -----------------------------------------------------------------------------
pub trait FxSpotMktSrc {
    fn get_fxspot_mkt(&self, pair: &CcyPair) -> anyhow::Result<FxSpotMkt>;
}
