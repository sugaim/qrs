use std::collections::HashMap;

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
    Number(f64),
    Int(i64),
    Boolean(bool),
    Object(HashMap<String, serde_json::Value>),
}

//
// methods
//
impl Component for Constant {
    #[inline]
    fn category(&self) -> ComponentCategory {
        match self {
            Constant::Number(_) => ComponentCategory::Constant(ValueType::Number),
            Constant::Int(_) => ComponentCategory::Constant(ValueType::Integer),
            Constant::Boolean(_) => ComponentCategory::Constant(ValueType::Boolean),
            Constant::Object(_) => ComponentCategory::Constant(ValueType::Object),
        }
    }

    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory)> {
        [].into_iter()
    }
}
