use std::fmt::{Debug, Display};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

// -----------------------------------------------------------------------------
// Component
//
pub trait Component {
    fn category(&self) -> ComponentCategory;
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory)>;
}

// -----------------------------------------------------------------------------
// ValueType
//
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, Display, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    #[strum(serialize = "number")]
    Number,
    #[strum(serialize = "integer")]
    Integer,
    #[strum(serialize = "boolean")]
    Boolean,
    #[strum(serialize = "object")]
    Object,
}

// -----------------------------------------------------------------------------
// ComponentCategory
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value_type")]
pub enum ComponentCategory {
    Constant(ValueType),
    Market,
    Process(ValueType),
    Cashflow,
    Leg,
}

impl Display for ComponentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentCategory::Constant(vt) => write!(f, "constant.{}", vt),
            ComponentCategory::Market => write!(f, "market"),
            ComponentCategory::Process(vt) => write!(f, "process.{}", vt),
            ComponentCategory::Cashflow => write!(f, "cashflow"),
            ComponentCategory::Leg => write!(f, "leg"),
        }
    }
}

// -----------------------------------------------------------------------------
// ComponentKey
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ComponentKey {
    pub cat: ComponentCategory,
    pub name: String,
}

//
// display, serde
//
impl Display for ComponentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.name, self.cat)
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

    type CashflowRef;
    type LegRef;
    type MarketRef;
    type ProcessRef;

    type CompoundingConvention;
}
