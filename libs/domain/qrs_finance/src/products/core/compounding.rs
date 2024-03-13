use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// CompoundingLockback
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompoundingLockback {
    #[serde(rename = "without_observation_shift")]
    WithoutObsShift { days: i32 },
    #[serde(rename = "observation_shift")]
    ObsShift { days: i32 },
    #[serde(rename = "weighted_observation_shift")]
    WeightedObsShift { days: i32 },
}

// -----------------------------------------------------------------------------
// CompoundingMethod
//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundingMethod {
    Straight,
    SpreadExclusive,
    Flat,
}

// -----------------------------------------------------------------------------
// CompoundingFloorTarget
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompoundingFloorTarget {
    Overall,
    EachRate,
}

// -----------------------------------------------------------------------------
// CompondingConvention
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CompoundingConvention<DayCount, Cal> {
    pub calendar: Cal,
    pub daycount: DayCount,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookback: Option<CompoundingLockback>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockout: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_target: Option<CompoundingFloorTarget>,
}
