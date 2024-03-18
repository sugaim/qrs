use schemars::{schema::SchemaObject, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::product::general::core::{Component, ComponentCategory};

use super::core::HasDependency;

// -----------------------------------------------------------------------------
// Constant
//
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Constant {
    Number(f64),
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
            serde_json::Value::Number(n) => Ok(Constant::Number(n.as_f64().unwrap())),
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
impl HasDependency for Constant {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = (&str, ComponentCategory)> {
        [].into_iter()
    }
}
impl Component for Constant {
    #[inline]
    fn category(&self) -> ComponentCategory {
        ComponentCategory::Constant
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(serde_json::json!(false), Constant::Boolean(false))]
    #[case(serde_json::json!(42.0), Constant::Number(42.0))]
    #[case(serde_json::json!("42"), Constant::String("42".to_string()))]
    #[case(serde_json::json!({"key": "value"}), Constant::Object(serde_json::json!({"key": "value"})))]
    fn test_deserialize(#[case] input: serde_json::Value, #[case] expected: Constant) {
        let constant: Constant = serde_json::from_value(input).unwrap();

        assert_eq!(constant, expected);
    }

    #[rstest]
    #[case(Constant::Boolean(false))]
    #[case(Constant::Number(42.0))]
    #[case(Constant::String("42".to_string()))]
    #[case(Constant::Object(serde_json::json!({"key": "value"})))]
    fn test_category(#[case] constant: Constant) {
        let cat = constant.category();

        assert_eq!(cat, ComponentCategory::Constant);
    }

    #[rstest]
    #[case(Constant::Boolean(false))]
    #[case(Constant::Number(42.0))]
    #[case(Constant::String("42".to_string()))]
    #[case(Constant::Object(serde_json::json!({"key": "value"})))]
    fn test_depends_on(#[case] constant: Constant) {
        let deps = constant.depends_on();

        assert!(deps.into_iter().next().is_none());
    }
}
