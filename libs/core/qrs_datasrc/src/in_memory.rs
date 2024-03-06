use std::{
    borrow::Borrow,
    collections::{
        hash_map::{IntoIter, IntoKeys, IntoValues, RandomState},
        HashMap,
    },
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut},
};

use anyhow::anyhow;
use derivative::Derivative;
use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, Response, Snapshot, TakeSnapshot};

// -----------------------------------------------------------------------------
// InMemoryDataSrc
//
/// Data source which possesses data in memory.
///
/// # Example
/// ```
/// use std::{collections::HashMap, string::ToString};
/// use qrs_datasrc::{DataSrc, CacheableSrc, InMemory};
///
/// let mut data = {
///     let mut data = HashMap::new();
///     data.insert("a".to_owned(), 1);
///     InMemory::from(data).with_etag_func(|s, v| format!("{}:{}", s, v))
/// };
///
/// let res = data.get_with_etag("a").unwrap();
/// assert_eq!(res.data, 1);
///
/// // None is returned because value for 'a' has not changed
/// let again = data.get_if_none_match("a", &res.etag).unwrap();
/// assert!(again.is_none());
///
/// // Some is returned because value for 'a' has changed
/// data.insert("a".to_owned(), 2);
/// let again = data.get_if_none_match("a", &res.etag).unwrap();
/// assert!(again.is_some());
/// assert_eq!(again.unwrap().data, 2);
/// ```
#[derive(Debug, Clone, Derivative, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "in-memory")]
#[derivative(
    PartialEq(bound = "K: Eq + Hash, V: PartialEq, S: BuildHasher, F: PartialEq"),
    Eq(bound = "K: Eq + Hash, V: Eq, S: BuildHasher, F: Eq")
)]
pub struct InMemory<K, V, S = RandomState, F = ()> {
    data: HashMap<K, V, S>,
    etag_gen: F,
}

//
// construction
//
impl<K, V, S: Default> Default for InMemory<K, V, S> {
    #[inline]
    fn default() -> Self {
        HashMap::default().into()
    }
}

impl<K, V> InMemory<K, V> {
    /// Create an empty in-memory data source.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an in-memory data source with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        HashMap::with_capacity(capacity).into()
    }
}

impl<K, V, S> InMemory<K, V, S> {
    /// Create an in-memory data source with the specified hasher.
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        HashMap::with_hasher(hash_builder).into()
    }

    /// Create an in-memory data source with the specified capacity and hasher.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        HashMap::with_capacity_and_hasher(capacity, hash_builder).into()
    }
}

impl<K, V, S, F> InMemory<K, V, S, F> {
    /// Set etag generator function to enable cache support.
    #[inline]
    pub fn with_etag_func<F2>(self, etag_gen: F2) -> InMemory<K, V, S, F2>
    where
        F2: Fn(&K, &V) -> String,
    {
        InMemory {
            data: self.data,
            etag_gen,
        }
    }
}

impl<K, V, S, F> InMemory<K, V, S, F>
where
    F: Fn(&K, &V) -> String,
{
    /// Remove etag generator function to disable cache support.
    #[inline]
    pub fn without_etag_func(self) -> InMemory<K, V, S> {
        InMemory {
            data: self.data,
            etag_gen: (),
        }
    }
}

impl<K, V, S> From<HashMap<K, V, S>> for InMemory<K, V, S> {
    #[inline]
    fn from(data: HashMap<K, V, S>) -> Self {
        InMemory { data, etag_gen: () }
    }
}

impl<K, V, S> FromIterator<(K, V)> for InMemory<K, V, S>
where
    K: Eq + Hash,
    S: Default + BuildHasher,
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        HashMap::from_iter(iter).into()
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for InMemory<K, V>
where
    K: Eq + Hash,
{
    #[inline]
    fn from(arr: [(K, V); N]) -> Self {
        HashMap::from(arr).into()
    }
}

//
// methods
//
impl<K, V, S, F> InMemory<K, V, S, F> {
    /// See [`HashMap::into_keys`].
    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        self.data.into_keys()
    }

    /// See [`HashMap::into_values`].
    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        self.data.into_values()
    }
}

impl<K, V, S, F> Deref for InMemory<K, V, S, F> {
    type Target = HashMap<K, V, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<K, V, S, F> DerefMut for InMemory<K, V, S, F> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<K, V, S, F> AsRef<HashMap<K, V, S>> for InMemory<K, V, S, F> {
    #[inline]
    fn as_ref(&self) -> &HashMap<K, V, S> {
        &self.data
    }
}

impl<K, V, S, F> AsMut<HashMap<K, V, S>> for InMemory<K, V, S, F> {
    #[inline]
    fn as_mut(&mut self) -> &mut HashMap<K, V, S> {
        &mut self.data
    }
}

impl<K, V, S, F> IntoIterator for InMemory<K, V, S, F> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, K, V, S, F> IntoIterator for &'a InMemory<K, V, S, F> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, K, V, S, F> IntoIterator for &'a mut InMemory<K, V, S, F> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = std::collections::hash_map::IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

impl<K, V, S, F, Q> DataSrc<Q> for InMemory<K, V, S, F>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
    S: BuildHasher,
    V: Clone,
{
    type Output = V;

    #[inline]
    fn get(&self, req: &Q) -> anyhow::Result<Self::Output> {
        self.data
            .get(req)
            .cloned()
            .ok_or_else(|| anyhow!("Key not found"))
    }
}

impl<K, V, S, F, Q> CacheableSrc<Q> for InMemory<K, V, S, F>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
    S: BuildHasher,
    V: Clone,
    F: Fn(&K, &V) -> String,
{
    #[inline]
    fn etag(&self, req: &Q) -> anyhow::Result<String> {
        self.data
            .get_key_value(req)
            .ok_or_else(|| anyhow!("Key not found"))
            .map(|(k, v)| (self.etag_gen)(k, v))
    }

    #[inline]
    fn get_with_etag(&self, req: &Q) -> anyhow::Result<Response<Self::Output>> {
        self.data
            .get_key_value(req)
            .ok_or_else(|| anyhow!("Key not found"))
            .map(|(k, v)| Response {
                etag: (self.etag_gen)(k, v),
                data: v.clone(),
            })
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Q,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        self.data
            .get_key_value(req)
            .ok_or_else(|| anyhow!("Key not found"))
            .map(|(k, v)| {
                let new_etag = (self.etag_gen)(k, v);
                if etag == new_etag {
                    None
                } else {
                    Some(Response {
                        etag: new_etag,
                        data: v.clone(),
                    })
                }
            })
    }
}

impl<K, V, S, Q> TakeSnapshot<Q> for InMemory<K, V, S>
where
    K: Eq + Hash + Borrow<Q> + Clone,
    Q: ?Sized + Eq + Hash,
    S: BuildHasher,
    V: Clone,
{
    type Snapshot = Snapshot<K, V>;

    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Q: 'a,
        Rqs: IntoIterator<Item = &'a Q>,
    {
        let data = rqs
            .into_iter()
            .map(|req| {
                self.data
                    .get_key_value(req)
                    .ok_or_else(|| anyhow!("Key not found"))
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            Response {
                                data: v.clone(),
                                etag: self.hasher().hash_one(k).to_string(),
                            },
                        )
                    })
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Snapshot::from(data))
    }
}

impl<K, V, S, F, Q> TakeSnapshot<Q> for InMemory<K, V, S, F>
where
    K: Eq + Hash + Borrow<Q> + Clone,
    Q: ?Sized + Eq + Hash,
    S: BuildHasher,
    V: Clone,
    F: Fn(&K, &V) -> String + Clone,
{
    type Snapshot = Snapshot<K, V>;

    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Q: 'a,
        Rqs: IntoIterator<Item = &'a Q>,
    {
        let data = rqs
            .into_iter()
            .map(|req| {
                self.data
                    .get_key_value(req)
                    .ok_or_else(|| anyhow!("Key not found"))
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            Response {
                                data: v.clone(),
                                etag: (self.etag_gen)(k, v),
                            },
                        )
                    })
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Snapshot::from(data))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::_test_util::*;

    #[fixture]
    fn src() -> InMemory<String, String> {
        hashmap! {
            "abc".to_owned() => "cba".to_owned(),
            "bcd".to_owned() => "dcb".to_owned(),
            "cde".to_owned() => "edc".to_owned(),
        }
        .into()
    }

    #[rstest]
    #[case("abc", Some(MockSrc::to_res("abc").data))]
    #[case(ERR_REQ, None)]
    fn test_get(
        src: InMemory<String, String>,
        #[case] input: &str,
        #[case] output: Option<String>,
    ) {
        let res = src.get(input);

        assert_eq!(
            res.map_err(|e| e.to_string()),
            output
                .map(|s| s.to_string())
                .ok_or("Key not found".to_owned())
        );
    }

    #[rstest]
    #[case("abc", Some(MockSrc::to_res("abc").etag))]
    #[case(ERR_REQ, None)]
    fn test_etag(#[case] input: &str, #[case] etag: Option<String>) {
        let src = src().with_etag_func(|k, _| MockSrc::to_etag(k));

        let res = src.etag(input);

        assert_eq!(
            res.map_err(|e| e.to_string()),
            etag.ok_or("Key not found".to_owned())
        );
    }

    #[rstest]
    #[case("abc", Some(MockSrc::to_res("abc")))]
    #[case(ERR_REQ, None)]
    fn test_get_with_etag(#[case] input: &str, #[case] output: Option<Response<String>>) {
        let src = src().with_etag_func(|k, _| MockSrc::to_etag(k));

        let res = src.get_with_etag(input);

        assert_eq!(
            res.map_err(|e| e.to_string()),
            output.ok_or("Key not found".to_owned())
        );
    }

    #[rstest]
    #[case("abc", MockSrc::to_etag("abc"), Ok(None))]
    #[case("abc", MockSrc::to_etag("xyz"), Ok(Some(MockSrc::to_res("abc"))))]
    #[case(ERR_REQ, MockSrc::to_etag("abc"), Err("Key not found".to_owned()))]
    fn test_get_if_none_match(
        #[case] input: &str,
        #[case] etag: String,
        #[case] output: Result<Option<Response<String>>, String>,
    ) {
        let src = src().with_etag_func(|k, _| MockSrc::to_etag(k));

        let res = src.get_if_none_match(input, &etag);

        assert_eq!(res.map_err(|e| e.to_string()), output);
    }

    #[rstest]
    #[case(&["abc", "bcd"])]
    #[case(&["abc", "bcd", "xyz"])]
    fn test_take_snapshot_as_datasrc(#[case] rqs: &[&str]) {
        let src = src();
        let is_ok = rqs.iter().all(|req| src.data.contains_key(*req));

        let snap = src.take_snapshot(rqs.iter().copied());

        if is_ok {
            let data = snap.unwrap().into_inner();
            assert_eq!(data.len(), rqs.len());
            for req in rqs {
                assert_eq!(data.get(*req).map(|r| r.data.clone()), src.get(*req).ok());
            }
        } else {
            assert!(snap.is_err());
        }
    }

    #[rstest]
    #[case(&["abc", "bcd"])]
    #[case(&["abc", "bcd", "xyz"])]
    fn test_take_snapshot_as_cacheable(#[case] rqs: &[&str]) {
        let src = src().with_etag_func(|k, _| MockSrc::to_etag(k));
        let is_ok = rqs.iter().all(|req| src.data.contains_key(*req));

        let snap = src.take_snapshot(rqs.iter().copied());

        if is_ok {
            let data = snap.unwrap().into_inner();
            assert_eq!(data.len(), rqs.len());
            for req in rqs {
                assert_eq!(data.get(*req).cloned(), src.get_with_etag(*req).ok());
            }
        } else {
            assert!(snap.is_err());
        }
    }
}
