use std::{
    borrow::Borrow,
    hash::Hash,
    ops::{Deref, Index, IndexMut},
};

use super::{impls::sealed, Error, SizedContainer, SplitFirst};

// -----------------------------------------------------------------------------
// SizeEnsured
// -----------------------------------------------------------------------------
/// A thin wrapper which guarantees that the length of the inner data is at least N at type level.
///
/// To calculate the length, it uses the [`ExactSizeIterator`] trait for the reference of the inner data.
/// That is, this struct requires the following bounds on the inner data `C`:
/// - `&'a C: IntoIterator`
/// - `<&'a C as IntoIterator>::IntoIter: ExactSizeIterator`
///
/// where `'a` is any lifetime which is greater than the lifetime of the inner data, `C: 'a`.
///
/// This struct implements [`Deref`], [`Borrow`], and [`AsRef`] to access the inner data immutablely.
/// On the other hand, to prevent changing the length of the inner data,
/// this struct does not expose inner data as mutable reference.
/// Only [`IntoIterator`] for the mutable reference or [`IndexMut`] is way to access the inner data as mutable.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SizeEnsured<C, const N: usize>(C);

pub type NonEmpty<C> = SizeEnsured<C, 1>;

//
// ctors
//
impl<C, const N: usize> SizeEnsured<C, N> {
    /// Construct a new instance.
    /// If the length of the data is less than N, it returns an error with the length.
    #[inline]
    pub fn new(data: C) -> Result<Self, Error>
    where
        C: SizedContainer,
    {
        let len = data.len();
        if len < N {
            Err(Error {
                required: N,
                actual: len,
            })
        } else {
            Ok(Self(data))
        }
    }
}

pub trait RequireMinSize<const N: usize>: Sized {
    fn require_min_size(self) -> Result<SizeEnsured<Self, N>, Error>;
}

impl<C: SizedContainer, const N: usize> RequireMinSize<N> for C {
    fn require_min_size(self) -> Result<SizeEnsured<Self, N>, Error> {
        SizeEnsured::new(self)
    }
}

//
// ser/de
//
impl<C, const N: usize> std::fmt::Display for SizeEnsured<C, N>
where
    C: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<C, const N: usize> serde::Serialize for SizeEnsured<C, N>
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

impl<'de, C, const N: usize> serde::Deserialize<'de> for SizeEnsured<C, N>
where
    C: serde::Deserialize<'de>,
    C: SizedContainer,
{
    fn deserialize<D>(deserializer: D) -> Result<SizeEnsured<C, N>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = C::deserialize(deserializer)?;
        SizeEnsured::new(data).map_err(serde::de::Error::custom)
    }
}

impl<C, const N: usize> schemars::JsonSchema for SizeEnsured<C, N>
where
    C: schemars::JsonSchema,
{
    fn schema_name() -> String {
        format!("SizeEnsured{}_for_{}", N, C::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qcollections::SizeEnsured<{}, {}>", C::schema_name(), N).into()
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
// methods
//
impl<C, const N: usize> Deref for SizeEnsured<C, N> {
    type Target = C;

    fn deref(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> Borrow<C> for SizeEnsured<C, N> {
    fn borrow(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> AsRef<C> for SizeEnsured<C, N> {
    fn as_ref(&self) -> &C {
        &self.0
    }
}

impl<C, const N: usize> IntoIterator for SizeEnsured<C, N>
where
    C: IntoIterator,
{
    type Item = <C as IntoIterator>::Item;
    type IntoIter = <C as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, C, const N: usize> IntoIterator for &'a SizeEnsured<C, N>
where
    &'a C: IntoIterator,
{
    type Item = <&'a C as IntoIterator>::Item;
    type IntoIter = <&'a C as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, C, const N: usize> IntoIterator for &'a mut SizeEnsured<C, N>
where
    &'a mut C: IntoIterator,
{
    type Item = <&'a mut C as IntoIterator>::Item;
    type IntoIter = <&'a mut C as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<C, const N: usize, Idx> Index<Idx> for SizeEnsured<C, N>
where
    C: Index<Idx>,
{
    type Output = C::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        &self.0[index]
    }
}

impl<C, const N: usize, Idx> IndexMut<Idx> for SizeEnsured<C, N>
where
    C: IndexMut<Idx>,
{
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<C, const N: usize> SizeEnsured<C, N> {
    /// Get the inner data.
    #[inline]
    pub fn inner(&self) -> &C {
        &self.0
    }

    /// Get the inner data.
    #[inline]
    pub fn into_inner(self) -> C {
        self.0
    }

    /// Get the first M elements.
    ///
    /// Because the length of the inner data is ensured to be at least N,
    /// this method can return the first M elements without checking the length.
    ///
    /// # Example
    /// ```
    /// use qcollections::size_ensured::{SizeEnsured, RequireMinSize};
    ///
    /// let data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();
    /// let (x0, x1, x2) = data.get_first().into();
    ///
    /// assert_eq!(x0, &1);
    /// assert_eq!(x1, &2);
    /// assert_eq!(x2, &3);
    /// ```
    #[inline]
    pub fn get_first<'a, const M: usize>(&'a self) -> [<&'a C as IntoIterator>::Item; M]
    where
        Self: sealed::Has<M>,
        &'a C: IntoIterator,
    {
        let indices = [0; M];
        let mut iter = self.0.into_iter();
        indices.map(|_| iter.next().expect("Must have enough elements"))
    }

    /// Get the first M elements.
    ///
    /// Mutable version of [`SizeEnsured::get_first`].
    #[inline]
    pub fn get_first_mut<'a, const M: usize>(&'a mut self) -> [<&'a mut C as IntoIterator>::Item; M]
    where
        Self: sealed::Has<M>,
        &'a mut C: IntoIterator,
    {
        let indices = [0; M];
        let mut iter = self.0.into_iter();
        indices.map(|_| iter.next().expect("Must have enough elements"))
    }

    /// Get the first M elements and the rest.
    ///
    /// Because the length of the inner data is ensured to be at least N,
    /// this method can return the first M elements without checking the length.
    ///
    /// # Example
    /// ```
    /// use qcollections::size_ensured::{SizeEnsured, RequireMinSize};
    ///
    /// let data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();
    /// let ((fst0, fst1, fst2), rest) = data.split_first().heads_into();
    ///
    /// assert_eq!(fst0, &1);
    /// assert_eq!(fst1, &2);
    /// assert_eq!(fst2, &3);
    /// assert_eq!(&rest.copied().collect::<Vec<_>>(), &[4, 5]);
    /// ```
    #[inline]
    pub fn split_first<'a, const M: usize>(
        &'a self,
    ) -> SplitFirst<<&'a C as IntoIterator>::Item, <&'a C as IntoIterator>::IntoIter, M>
    where
        Self: sealed::Has<M>,
        &'a C: IntoIterator,
        <&'a C as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let indices = [0; M];
        let mut iter = self.0.into_iter();
        SplitFirst {
            heads: indices.map(|_| iter.next().expect("Must have enough elements")),
            tails: iter,
        }
    }

    /// Get the first M elements and the rest with mutable reference.
    ///
    /// Mutable version of [`SizeEnsured::split_first`].
    #[inline]
    pub fn split_first_mut<'a, const M: usize>(
        &'a mut self,
    ) -> SplitFirst<<&'a mut C as IntoIterator>::Item, <&'a mut C as IntoIterator>::IntoIter, M>
    where
        Self: sealed::Has<M>,
        &'a mut C: IntoIterator,
        <&'a mut C as IntoIterator>::IntoIter: ExactSizeIterator,
    {
        let indices = [0; M];
        let mut iter = self.0.into_iter();
        SplitFirst {
            heads: indices.map(|_| iter.next().expect("Must have enough elements")),
            tails: iter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctor() {
        let data: Vec<usize> = vec![1, 2, 3, 4, 5];
        let tested: Result<SizeEnsured<_, 6>, _> = data.clone().require_min_size();

        assert!(tested.is_err());
        let err = tested.unwrap_err();
        assert_eq!(err.required, 6);
        assert_eq!(err.actual, 5);

        let tested: SizeEnsured<_, 5> = data.clone().require_min_size().unwrap();
        assert_eq!(tested.len(), 5);
        assert_eq!(tested.inner(), &data);

        let tested: NonEmpty<_> = data.clone().require_min_size().unwrap();
        assert_eq!(tested.len(), 5);
        assert_eq!(&tested.into_inner(), &data);
    }

    #[test]
    fn test_get_first() {
        let data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();

        //
        let (x0,) = data.get_first().into();

        assert_eq!(x0, &1);

        //
        let (x0, x1, x2) = data.get_first().into();

        assert_eq!(x0, &1);
        assert_eq!(x1, &2);
        assert_eq!(x2, &3);
    }

    #[test]
    fn test_get_first_mut() {
        let mut data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();

        //
        let (x0,) = data.get_first_mut().into();
        *x0 += 1;

        assert_eq!(x0, &2);

        //
        let (x0, x1, x2) = data.get_first_mut().into();
        *x0 += 1;
        *x1 += 1;
        *x2 += 1;

        assert_eq!(x0, &3);
        assert_eq!(x1, &3);
        assert_eq!(x2, &4);
    }

    #[test]
    fn test_split_first() {
        let data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();

        //
        let ((fst0,), rest) = data.split_first().heads_into();

        assert_eq!(fst0, &1);
        assert_eq!(&rest.copied().collect::<Vec<_>>(), &[2, 3, 4, 5]);

        //
        let ((fst0, fst1, fst2), rest) = data.split_first().heads_into();

        assert_eq!(fst0, &1);
        assert_eq!(fst1, &2);
        assert_eq!(fst2, &3);
        assert_eq!(&rest.copied().collect::<Vec<_>>(), &[4, 5]);
    }

    #[test]
    fn test_split_first_mut() {
        let mut data: SizeEnsured<Vec<usize>, 3> = vec![1, 2, 3, 4, 5].require_min_size().unwrap();

        //
        let ((fst0,), rest) = data.split_first_mut().heads_into();
        *fst0 += 1;

        let add_ret = |x: &mut usize| {
            *x += 2;
            *x
        };
        assert_eq!(fst0, &2);
        assert_eq!(&rest.map(add_ret).collect::<Vec<_>>(), &[4, 5, 6, 7]);

        //
        let ((fst0, fst1, fst2), rest) = data.split_first_mut().heads_into();
        *fst0 += 1;
        *fst1 += 1;
        *fst2 += 1;

        assert_eq!(fst0, &3);
        assert_eq!(fst1, &5);
        assert_eq!(fst2, &6);
        assert_eq!(&rest.map(add_ret).collect::<Vec<_>>(), &[8, 9]);
    }
}
