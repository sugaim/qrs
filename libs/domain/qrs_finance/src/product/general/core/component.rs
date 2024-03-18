use std::fmt::{Debug, Display};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Id;

// -----------------------------------------------------------------------------
// HasDependency
//
pub(crate) trait HasDependency {
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory)>;
}

// -----------------------------------------------------------------------------
// Component
//
pub(crate) trait Component: HasDependency {
    fn category(&self) -> ComponentCategory;
}

// -----------------------------------------------------------------------------
// ComponentCategory
//
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value_type")]
pub enum ComponentCategory {
    #[strum(serialize = "constant")]
    Constant,
    #[strum(serialize = "market")]
    Market,
    #[strum(serialize = "process")]
    Process,
    #[strum(serialize = "cashflow")]
    Cashflow,
    #[strum(serialize = "leg")]
    Leg,
}

// -----------------------------------------------------------------------------
// ComponentKey
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ComponentKey {
    pub cat: ComponentCategory,
    pub id: Id,
}

//
// display, serde
//
impl Display for ComponentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.id, self.cat)
    }
}

// -----------------------------------------------------------------------------
// VariableTypes
//
pub trait VariableTypes {
    type Number;
    type Integer;
    type Boolean;

    type DateTime;
    type DayCount;
    type Calendar;
    type Rounding;
    type Money;

    type CashflowRef;
    type LegRef;
    type MarketRef;
    type ProcessRef;

    type InArrearsConvention;
}
