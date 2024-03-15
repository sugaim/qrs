use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::{Component, ComponentCategory};

// -----------------------------------------------------------------------------
// Constant
//
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(untagged)]
pub enum Constant {
    Number(f64),
    Int(i64),
    Boolean(bool),
    String(String),
    Object(serde_json::Value),
}

impl<'de> Deserialize<'de> for Constant {
    fn deserialize<D>(deserializer: D) -> Result<Constant, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => {
                if let Some(n) = n.as_i64() {
                    Ok(Constant::Int(n))
                } else {
                    Ok(Constant::Number(n.as_f64().unwrap()))
                }
            }
            serde_json::Value::Bool(b) => Ok(Constant::Boolean(b)),
            serde_json::Value::String(s) => Ok(Constant::String(s)),
            serde_json::Value::Object(_) => Ok(Constant::Object(value)),
            _ => Err(serde::de::Error::custom(
                "Invalid constant value. Only numbers, booleans and objects are allowed.",
            )),
        }
    }
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
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory)> {
        [].into_iter()
    }
}
