use schemars::{schema::SchemaObject, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::products::general::core::{Component, ComponentCategory};

// -----------------------------------------------------------------------------
// Constant
//
#[derive(Debug, Clone, PartialEq, Serialize)]
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

impl JsonSchema for Constant {
    fn schema_name() -> String {
        "Constant".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_finance::product::general::core::Constant".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut sch = SchemaObject::default();
        sch.subschemas().one_of = Some(vec![
            f64::json_schema(gen),
            i64::json_schema(gen),
            bool::json_schema(gen),
            String::json_schema(gen),
            serde_json::Map::<String, serde_json::Value>::json_schema(gen),
        ]);
        sch.metadata().description = Some("Constant value refered from contract data".to_string());
        sch.into()
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
