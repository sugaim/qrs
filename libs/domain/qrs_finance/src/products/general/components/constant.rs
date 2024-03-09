use crate::products::general::{Component, ComponentCategory, ValueType};

// -----------------------------------------------------------------------------
// Constant
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(untagged)
)]
pub enum Constant {
    Float(f64),
    Integer(i64),
    Boolean(bool),
}

//
// methods
//
impl Component for Constant {
    #[inline]
    fn category(&self) -> ComponentCategory {
        ComponentCategory::Constant
    }

    #[inline]
    fn value_type(&self) -> ValueType {
        match self {
            Constant::Float(_) => ValueType::Float,
            Constant::Integer(_) => ValueType::Integer,
            Constant::Boolean(_) => ValueType::Boolean,
        }
    }

    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory, ValueType)> {
        [].into_iter()
    }
}
