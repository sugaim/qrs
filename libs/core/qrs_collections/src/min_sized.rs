use std::{borrow::Borrow, ops::Deref};

// -----------------------------------------------------------------------------
// MinSizedError
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, thiserror::Error)]
pub enum MinSizedError {
    #[error("Size is {}, which is less than required size {}", .actual, .required)]
    TooSmall { required: usize, actual: usize },
}

// -----------------------------------------------------------------------------
// MinSized
//
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MinSized<C, const N: usize>(C);

pub type NonEmpty<C> = MinSized<C, 1>;

//
// display, serde
//
impl<C, const N: usize> std::fmt::Display for MinSized<C, N>
where
    C: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "serde")]
impl<C, const N: usize> serde::Serialize for MinSized<C, N>
where
    C: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, C, const N: usize> serde::Deserialize<'de> for MinSized<C, N>
where
    C: serde::Deserialize<'de>,
    C: ExactSizeIter,
{
    fn deserialize<D>(deserializer: D) -> Result<MinSized<C, N>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = C::deserialize(deserializer)?;
        MinSized::new(data).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<C, const N: usize> schemars::JsonSchema for MinSized<C, N>
where
    C: schemars::JsonSchema,
{
    fn schema_name() -> String {
        format!("MinSized{}_for_{}", N, C::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_collections::MinSized<{}, {}>", C::schema_name(), N).into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{InstanceType, Schema, SingleOrVec};

        let res = C::json_schema(gen);
        let Schema::Object(mut obj) = res else {
            return res;
        };
        let (is_string, is_array, is_object) = match &obj.instance_type {
            Some(SingleOrVec::Single(inst_type)) => {
                let inst_type = inst_type.as_ref();
                (
                    inst_type == &InstanceType::String,
                    inst_type == &InstanceType::Array,
                    inst_type == &InstanceType::Object,
                )
            }
            Some(SingleOrVec::Vec(inst_types)) => (
                inst_types.contains(&InstanceType::String),
                inst_types.contains(&InstanceType::Array),
                inst_types.contains(&InstanceType::Object),
            ),
            _ => (false, false, false),
        };
        if is_string {
            obj.string().min_length = Some(N as _);
        }
        if is_array {
            obj.array().min_items = Some(N as _);
        }
        if is_object {
            obj.object().min_properties = Some(N as _);
        }
        Schema::Object(obj)
    }
}

//
// construction
//
impl<C, const N: usize> MinSized<C, N> {
    /// Construct a new instance.
    /// If the length of the data is less than N, it returns an error with the length.
    #[inline]
    pub fn new(data: C) -> Result<Self, MinSizedError>
    where
        C: ExactSizeIter,
    {
        let len = data.len();
        if len < N {
            Err(MinSizedError::TooSmall {
                required: N,
                actual: len,
            })
        } else {
            Ok(Self(data))
        }
    }
}

pub trait RequireMinSize<const N: usize>: Sized {
    fn require_min_size(self) -> Result<MinSized<Self, N>, MinSizedError>;
}

impl<C: ExactSizeIter, const N: usize> RequireMinSize<N> for C {
    fn require_min_size(self) -> Result<MinSized<Self, N>, MinSizedError> {
        MinSized::new(self)
    }
}

//
// methods
//
impl<C, const N: usize> Deref for MinSized<C, N> {
    type Target = C;

    fn deref(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> Borrow<C> for MinSized<C, N> {
    fn borrow(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> AsRef<C> for MinSized<C, N> {
    fn as_ref(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> MinSized<C, N> {
    /// Get the inner data.
    #[inline]
    pub fn into_inner(self) -> C {
        self.0
    }
}
impl<C, const N: usize> IntoIterator for MinSized<C, N>
where
    C: IntoIterator,
{
    type Item = <C as IntoIterator>::Item;
    type IntoIter = <C as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub trait RefIntoIter<'this, ImplicitBound: sealed::Sealed = sealed::Bound<&'this Self>> {
    type Iter: ExactSizeIterator;
}

impl<'this, I> RefIntoIter<'this> for I
where
    &'this I: IntoIterator,
    <&'this I as IntoIterator>::IntoIter: ExactSizeIterator,
{
    type Iter = <&'this I as IntoIterator>::IntoIter;
}

pub trait ExactSizeIter: for<'this> RefIntoIter<'this> {
    fn iter(&self) -> <Self as RefIntoIter<'_>>::Iter;
    fn len(&self) -> usize {
        self.iter().len()
    }
}

impl<I> ExactSizeIter for I
where
    for<'this> &'this I: IntoIterator,
    for<'this> <&'this I as IntoIterator>::IntoIter: ExactSizeIterator,
{
    fn iter(&self) -> <Self as RefIntoIter<'_>>::Iter {
        self.into_iter()
    }
}

mod sealed {
    pub trait Sealed {}
    pub struct Bound<T>(T);
    impl<T> Sealed for Bound<T> {}

    pub trait Has<const N: usize> {}

    macro_rules! impl_has {
        ($n:expr; $($m:expr),*) => {
            $(
                impl<T> Has<$m> for super::MinSized<T, $n> {}
            )*
        };
    }

    impl_has!(1; 1);
    impl_has!(2; 1, 2);
    impl_has!(3; 1, 2, 3);
    impl_has!(4; 1, 2, 3, 4);
    impl_has!(5; 1, 2, 3, 4, 5);
    impl_has!(6; 1, 2, 3, 4, 5, 6);
    impl_has!(7; 1, 2, 3, 4, 5, 6, 7);
    impl_has!(8; 1, 2, 3, 4, 5, 6, 7, 8);
}

impl<C, const N: usize> MinSized<C, N> {
    /// Get the first element.
    pub fn get1<'a>(&'a self) -> <&'a C as IntoIterator>::Item
    where
        Self: sealed::Has<1>,
        &'a C: IntoIterator,
    {
        self.0.into_iter().next().unwrap()
    }

    /// Get the first element and the rest.
    pub fn split_first1<'a>(
        &'a self,
    ) -> (
        <&'a C as IntoIterator>::Item,
        <&'a C as IntoIterator>::IntoIter,
    )
    where
        Self: sealed::Has<1>,
        &'a C: IntoIterator,
        <&'a C as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let mut iter = self.0.into_iter();
        let first = iter.next().unwrap();
        (first, iter)
    }

    /// Get the first two elements.
    pub fn get2<'a>(&'a self) -> (<&'a C as IntoIterator>::Item, <&'a C as IntoIterator>::Item)
    where
        Self: sealed::Has<2>,
        &'a C: IntoIterator,
    {
        let mut iter = self.0.into_iter();
        (iter.next().unwrap(), iter.next().unwrap())
    }

    /// Get the first two elements and the rest.
    pub fn split_first2<'a>(
        &'a self,
    ) -> (
        <&'a C as IntoIterator>::Item,
        <&'a C as IntoIterator>::Item,
        <&'a C as IntoIterator>::IntoIter,
    )
    where
        Self: sealed::Has<2>,
        &'a C: IntoIterator,
        <&'a C as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let mut iter = self.0.into_iter();
        let first = iter.next().unwrap();
        let second = iter.next().unwrap();
        (first, second, iter)
    }
}
