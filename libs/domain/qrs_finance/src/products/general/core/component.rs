use std::fmt::Display;

// -----------------------------------------------------------------------------
// Component
//
pub trait Component {
    fn category(&self) -> ComponentCategory;
    fn value_type(&self) -> ValueType;
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory, ValueType)>;
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
    #[strum(serialize = "float")]
    Float,
    #[strum(serialize = "integer")]
    Integer,
    #[strum(serialize = "boolean")]
    Boolean,
}

// -----------------------------------------------------------------------------
// ComponentCategory
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::EnumString, strum::Display)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case")
)]
pub enum ComponentCategory {
    #[strum(serialize = "constant")]
    Constant,
    #[strum(serialize = "process")]
    Process,
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
