use schemars::{schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use super::ComponentField;

// -----------------------------------------------------------------------------
// Id
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub String);

//
// display, serde
//
impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$ref:{}", self.0)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("$ref:{}", self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(id) = s.strip_prefix("$ref:") {
            Ok(Id(id.to_string()))
        } else {
            Err(serde::de::Error::custom("Invalid id format"))
        }
    }
}

impl JsonSchema for Id {
    fn schema_name() -> String {
        "Id".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_finance::product::general::core::Id".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = String::json_schema(gen);
        if let Schema::Object(ref mut obj) = schema {
            obj.metadata().description = Some("Id of a component".to_string());
            obj.string().pattern = Some(r"^\$ref:.+$".to_string());
        }
        schema
    }
}

//
// construction
//
impl AsRef<str> for Id {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Id {
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Id(id.into())
    }
}

impl From<&str> for Id {
    #[inline]
    fn from(id: &str) -> Self {
        Id(id.to_string())
    }
}

impl From<String> for Id {
    #[inline]
    fn from(id: String) -> Self {
        Id(id)
    }
}

impl From<Id> for String {
    #[inline]
    fn from(id: Id) -> String {
        id.0
    }
}

//
// methods
//
impl ComponentField for Id {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        [self.0.as_str()].into_iter()
    }
}

// -----------------------------------------------------------------------------
// ValueOrId
//
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(untagged)]
pub enum ValueOrId<T> {
    Id(Id),
    Value(T),
}

//
// methods
//
impl<T> ComponentField for ValueOrId<T> {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        enum Either<L, R> {
            Left(L),
            Right(R),
        }
        impl<L, R> Iterator for Either<L, R>
        where
            L: Iterator,
            R: Iterator<Item = L::Item>,
        {
            type Item = L::Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Either::Left(l) => l.next(),
                    Either::Right(r) => r.next(),
                }
            }
        }
        match self {
            ValueOrId::Value(_) => Either::Left([].into_iter()),
            ValueOrId::Id(id) => Either::Right(id.depends_on().into_iter()),
        }
    }
}

// -----------------------------------------------------------------------------
// ValueLess
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueLess;

// -----------------------------------------------------------------------------
// WithId
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct WithId<T> {
    pub id: Id,
    pub value: T,
}

//
// display, serde
//
impl Serialize for WithId<ValueLess> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.id.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WithId<ValueLess> {
    fn deserialize<D>(deserializer: D) -> Result<WithId<ValueLess>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(WithId {
            id: Id::deserialize(deserializer)?,
            value: ValueLess,
        })
    }
}

impl JsonSchema for WithId<ValueLess> {
    fn schema_name() -> String {
        Id::schema_name()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        Id::schema_id()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        Id::json_schema(gen)
    }
}

//
// methods
//
impl<T> WithId<T> {
    #[inline]
    pub fn map<O>(self, f: impl FnOnce(T) -> O) -> WithId<O> {
        WithId {
            id: self.id,
            value: f(self.value),
        }
    }
}

impl ComponentField for WithId<ValueLess> {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.id.depends_on()
    }
}

impl<T: ComponentField> ComponentField for WithId<T> {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.value.depends_on()
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_or_id_depends_on() {
        let value: ValueOrId<ValueLess> = ValueOrId::Value(ValueLess);
        assert!(value.depends_on().into_iter().next().is_none());

        let id: ValueOrId<ValueLess> = ValueOrId::Id("id".into());
        assert_eq!(id.depends_on().into_iter().collect::<Vec<_>>(), vec!["id"]);
    }
}
