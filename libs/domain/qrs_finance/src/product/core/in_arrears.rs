use qrs_math::rounding::Rounding;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Lookback
//
/// Lookback convention
///
/// With `n` day lookback, rate applied on today is a observed rate `n` days ago.
///
/// If tomorrow is Friday, the rate is applied three days(today, tomorrow, and the day after tomorrow).
/// (For simplicity, we assume that we do not have any special holidays around today)
///
/// But with observation shift, this date counting is also shifted in addition to applied rate.
/// That is, in 2 days lookback with days observation shift, the number of days is counted 1
/// because date counting is also started from Wednesday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Lookback {
    #[serde(rename = "without_observation_shift")]
    WithoutObsShift { days: i32 },

    #[serde(rename = "observation_shift")]
    ObsShift { days: i32 },
}

// -----------------------------------------------------------------------------
// StraightCompounding
//
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StraightCompounding<DayCount, Cal> {
    /// Calendar for rate publication.
    pub rate_calendar: Cal,

    /// Day count convention used to calculate dcf of each observation period.
    pub obsrate_daycount: DayCount,

    /// Day count convention used to calculate dcf of accrual period.
    pub overall_daycount: DayCount,

    /// Lookback convention
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookback: Option<Lookback>,

    /// Lockout period
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockout: Option<i32>,

    /// Apply zero floor on each observed rate.
    #[serde(default)]
    pub zero_interest_rate_method: bool,

    /// Rounding for compunded rate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Rounding>,
}

// -----------------------------------------------------------------------------
// SpreadExclusiveCompounding
//
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpreadExclusiveCompounding<DayCount, Cal> {
    /// Calendar for rate publication.
    pub rate_calendar: Cal,

    /// Day count convention used to calculate dcf of each observation period.
    pub obsrate_daycount: DayCount,

    /// Day count convention used to calculate dcf of accrual period.
    pub overall_daycount: DayCount,

    /// Lookback convention
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookback: Option<Lookback>,

    /// Lockout period
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockout: Option<i32>,

    /// Apply zero floor on each observed rate.
    #[serde(default)]
    pub zero_interest_rate_method: bool,

    /// Rounding for compunded rate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Rounding>,
}

// -----------------------------------------------------------------------------
// InArrears
//
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InArrears<DayCount, Cal> {
    Straight(StraightCompounding<DayCount, Cal>),
    SpreadExclusive(SpreadExclusiveCompounding<DayCount, Cal>),
}
