// -----------------------------------------------------------------------------
// CompoundingLockback
//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum CompoundingLockback {
    #[cfg_attr(feature = "serde", serde(rename = "without_observation_shift"))]
    WithoutObsShift { days: i32 },
    #[cfg_attr(feature = "serde", serde(rename = "observation_shift"))]
    ObsShift { days: i32 },
    #[cfg_attr(feature = "serde", serde(rename = "weighted_observation_shift"))]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case")
)]
pub enum CompoundingFloorTarget {
    Overall,
    EachRate,
}

// -----------------------------------------------------------------------------
// CompondingConvention
//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct CompoundingConvention<DayCount, Cal> {
    pub calendar: Cal,
    pub daycount: DayCount,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lookback: Option<CompoundingLockback>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lockout: Option<i32>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub floor_target: Option<CompoundingFloorTarget>,
}
