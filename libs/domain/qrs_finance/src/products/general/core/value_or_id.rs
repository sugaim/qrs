use schemars::JsonSchema;
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
