use chrono::Datelike;

use crate::timepoint::Date;

use super::Calendar;

// -----------------------------------------------------------------------------
// HolidayAdj
// -----------------------------------------------------------------------------
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    strum::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum HolidayAdj {
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
}

impl HolidayAdj {
    /// Adjust the date according to the holiday adjustment rule.
    ///
    /// This returns [None] if the date is out of supported range.
    pub fn adjust(&self, d: Date, cal: &Calendar) -> Option<Date> {
        if cal.is_bizday(d).ok()? {
            return Some(d);
        }
        match self {
            HolidayAdj::Following => cal.iter_bizdays(d).next(),
            HolidayAdj::ModifiedFollowing => {
                let nxt = cal.iter_bizdays(d).next()?;
                if nxt.month() == d.month() {
                    Some(nxt)
                } else {
                    HolidayAdj::Preceding.adjust(d, cal)
                }
            }
            HolidayAdj::Preceding => cal.iter_bizdays(d).next_back(),
            HolidayAdj::ModifiedPreceding => {
                let prev = cal.iter_bizdays(d).next_back()?;
                if prev.month() == d.month() {
                    Some(prev)
                } else {
                    HolidayAdj::Following.adjust(d, cal)
                }
            }
        }
    }
}
