use std::fmt::Debug;

use qrs_chrono::{CalendarSymbol, DateWithTag};

use crate::core::daycount::DayCountSymbol;

// -----------------------------------------------------------------------------
// ValueOrId
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(untagged)
)]
pub enum ValueOrId<T> {
    Value(T),
    Id(String),
}

// -----------------------------------------------------------------------------
// VariableTypes
//
pub trait VariableTypes {
    type Number: Debug + Clone + PartialEq;
    type Int: Debug + Clone + Eq;
    type Boolean: Debug + Clone + Eq;

    type DateTime: Debug + Clone + Eq + std::hash::Hash;
    type DayCount: Debug + Clone + Eq;
    type Calendar: Debug + Clone + Eq;

    type CashflowRef: Debug + Clone + PartialEq;
    type LegRef: Debug + Clone + PartialEq;
    type MarketRef: Debug + Clone + PartialEq;
    type ProcessRef: Debug + Clone + PartialEq;

    type General<T>: Debug + Clone + PartialEq
    where
        T: Debug + Clone + PartialEq;
}

// -----------------------------------------------------------------------------
// VariableTypesForParse
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct VariableTypesForParse;

//
// methods
//
impl VariableTypes for VariableTypesForParse {
    type Number = ValueOrId<f64>;
    type Int = ValueOrId<i64>;
    type Boolean = ValueOrId<bool>;

    type DateTime = DateWithTag;
    type DayCount = DayCountSymbol;
    type Calendar = CalendarSymbol;

    type CashflowRef = String;
    type LegRef = String;
    type MarketRef = String;
    type ProcessRef = String;

    type General<T> = ValueOrId<T>
    where
        T: Debug + Clone + PartialEq;
}
