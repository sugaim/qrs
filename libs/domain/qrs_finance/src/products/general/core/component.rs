use std::fmt::Display;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::Display)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case")
)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case", tag = "type", content = "value_type")
)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
