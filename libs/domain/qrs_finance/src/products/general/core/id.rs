use schemars::{schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use super::ComponentField;

// -----------------------------------------------------------------------------
// ValueOrId
//
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(untagged)]
pub enum ValueOrId<T> {
    Value(T),
    Id(String),
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
    pub id: String,
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
            id: String::deserialize(deserializer)?,
            value: ValueLess,
        })
    }
}

impl JsonSchema for WithId<ValueLess> {
    fn schema_name() -> String {
        "WithId".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_finance::product::general::core::WithId<ValueLess>".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = String::json_schema(gen);
        if let Schema::Object(ref mut obj) = schema {
            obj.metadata().description = Some("Id of a component".to_string());
        }
        schema
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
        [self.id.as_str()].into_iter()
    }
}

impl<T: ComponentField> ComponentField for WithId<T> {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        self.value.depends_on()
    }
}
