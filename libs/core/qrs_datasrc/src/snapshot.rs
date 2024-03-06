use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use derivative::Derivative;
use qrs_datasrc_derive::DebugTree;

use crate::{CacheableSrc, DataSrc, Response};

// -----------------------------------------------------------------------------
// TakeSnapshot
//
/// A trait for taking a snapshot of the data source.
///
/// Implementations must ensure that the snapshot is independent on the state of
/// the original data source.
/// For example, even though the original data source is a real-time data source,
/// the snapshot must be static.
///
/// This maybe useful for calculation of financial risks with bump-and-revalue method.
pub trait TakeSnapshot<Rq: ?Sized>: DataSrc<Rq> {
    type Snapshot: DataSrc<Rq, Output = Self::Output>;

    /// Take a snapshot of the data source.
    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Rq: 'a,
        Rqs: IntoIterator<Item = &'a Rq>;
}

impl<S, Rq: ?Sized> TakeSnapshot<Rq> for Arc<S>
where
    S: TakeSnapshot<Rq> + ?Sized,
{
    type Snapshot = S::Snapshot;

    #[inline]
    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Rq: 'a,
        Rqs: IntoIterator<Item = &'a Rq>,
    {
        self.as_ref().take_snapshot(rqs)
    }
}

impl<S, Rq: ?Sized> TakeSnapshot<Rq> for Mutex<S>
where
    S: TakeSnapshot<Rq> + ?Sized,
{
    type Snapshot = S::Snapshot;

    #[inline]
    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Rq: 'a,
        Rqs: IntoIterator<Item = &'a Rq>,
    {
        self.lock().unwrap().take_snapshot(rqs)
    }
}

// -----------------------------------------------------------------------------
// Snapshot
//
/// A naive implementation of a snapshot.
///
/// This is just a thin wrapper of a hash map with a data source interface.
/// Due to this, only data sources whose request type is [`Eq`] and [`Hash`] can be used.
///
/// There are two ways to construct a snapshot.
/// - [`Snapshot::with_datasrc`]
/// - [`Snapshot::with_cacheable`]
/// These two are not so different.
/// The formet is for [`DataSrc`] and calculate etags by hash of the request.
/// (Since this object is immutable, value corresponding to the request is immutable and
/// the hash value of the key works as the etag of the value.)
/// The latter is for [`CacheableSrc`] and calculate etags based on the logics of the data source.
#[derive(Debug, Clone, Derivative, DebugTree)]
#[debug_tree(_use_from_qrs_datasrc, desc = "snapshot")]
#[derivative(
    PartialEq(bound = "K: Eq + Hash, V: PartialEq"),
    Eq(bound = "K: Eq + Hash, V: Eq")
)]
pub struct Snapshot<K, V> {
    data: HashMap<K, Response<V>>,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<K, V> serde::Serialize for Snapshot<K, V>
where
    K: serde::Serialize,
    V: serde::Serialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::{ser::SerializeSeq, Serialize, Serializer};

        struct Data<'a, K, V>(&'a HashMap<K, Response<V>>);

        impl<'a, K, V> serde::Serialize for Data<'a, K, V>
        where
            K: Serialize,
            V: Serialize,
        {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                #[derive(Serialize)]
                struct Item<'a, K, V> {
                    key: &'a K,
                    value: &'a V,
                    etag: &'a str,
                }
                let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
                for (k, v) in self.0.iter() {
                    seq.serialize_element(&Item {
                        key: k,
                        value: &v.data,
                        etag: &v.etag,
                    })?;
                }
                seq.end()
            }
        }
        Data(&self.data).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::Deserialize<'de> for Snapshot<K, V>
where
    K: serde::Deserialize<'de> + Eq + Hash,
    V: serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Snapshot<K, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::{
            de::{SeqAccess, Visitor},
            Deserialize, Deserializer,
        };
        use std::marker::PhantomData;

        struct Data<K, V>(HashMap<K, Response<V>>);
        struct Vst<K, V>(PhantomData<(K, V)>);

        impl<'de, K, V> Deserialize<'de> for Data<K, V>
        where
            K: Deserialize<'de> + Eq + Hash,
            V: Deserialize<'de>,
        {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(Vst(PhantomData))
            }
        }

        impl<'de, K, V> Visitor<'de> for Vst<K, V>
        where
            K: Deserialize<'de> + Eq + Hash,
            V: Deserialize<'de>,
        {
            type Value = Data<K, V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a data for data source")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                #[derive(Deserialize)]
                struct Item<K, V> {
                    key: K,
                    value: V,
                    etag: Option<String>,
                }
                let mut res: HashMap<K, Response<V>> =
                    HashMap::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(Item { key, value, etag }) = seq.next_element::<Item<K, V>>()? {
                    let etag = etag.unwrap_or_else(|| {
                        // snapshot is immutable. hence, we can use the hash value of key, instead of value
                        res.hasher().hash_one(&key).to_string()
                    });
                    res.insert(key, Response { data: value, etag });
                }
                Ok(Data(res))
            }
        }

        let data = Data::<K, V>::deserialize(deserializer)?;
        Ok(Snapshot { data: data.0 })
    }
}

//
// construction
//
impl<K, V> Snapshot<K, V> {
    /// Create a snapshot from a data source and a set of requests.
    ///
    /// The etag of each response is calculated by the hash of the request.
    /// If the data source is [`CacheableSrc`], please use [`Snapshot::with_cacheable`]
    /// because it uses logic of the original data source to calculate etags.
    pub fn with_datasrc<'a, Rq>(
        src: &impl DataSrc<Rq, Output = V>,
        rqs: impl IntoIterator<Item = &'a Rq>,
    ) -> anyhow::Result<Self>
    where
        Rq: 'a + ?Sized + Eq + Hash + ToOwned<Owned = K>,
        K: Eq + Hash + Borrow<Rq>,
    {
        let rqs = rqs.into_iter();
        let mut data = match rqs.size_hint() {
            (lower, None) => HashMap::with_capacity(lower),
            (lower, Some(upper)) => HashMap::with_capacity((upper + lower) / 2),
        };
        for rq in rqs {
            let res = src.get(rq)?;
            let key = rq.to_owned();
            let etag = data.hasher().hash_one(&key).to_string();
            data.insert(key, Response { data: res, etag });
        }
        Ok(Snapshot { data })
    }

    /// Create a snapshot from a data source and a set of requests.
    ///
    /// The etag of each response is calculated by the logic of the original data source.
    pub fn with_cacheable<'a, Rq>(
        src: &impl CacheableSrc<Rq, Output = V>,
        rqs: impl IntoIterator<Item = &'a Rq>,
    ) -> anyhow::Result<Self>
    where
        Rq: 'a + ?Sized + Eq + Hash + ToOwned<Owned = K>,
        K: Eq + Hash + Borrow<Rq>,
    {
        let rqs = rqs.into_iter();
        let mut data = match rqs.size_hint() {
            (lower, None) => HashMap::with_capacity(lower),
            (lower, Some(upper)) => HashMap::with_capacity((upper + lower) / 2),
        };
        for rq in rqs {
            let res = src.get_with_etag(rq)?;
            data.insert(rq.to_owned(), res);
        }
        Ok(Snapshot { data })
    }
}

impl<K, V> From<HashMap<K, Response<V>>> for Snapshot<K, V> {
    #[inline]
    fn from(data: HashMap<K, Response<V>>) -> Self {
        Snapshot { data }
    }
}

//
// methods
//
impl<K, V> Snapshot<K, V> {
    /// Get the inner data.
    #[inline]
    pub fn inner(&self) -> &HashMap<K, Response<V>> {
        &self.data
    }

    /// Unwrap the inner data.
    #[inline]
    pub fn into_inner(self) -> HashMap<K, Response<V>> {
        self.data
    }
}

impl<K, V, Rq> DataSrc<Rq> for Snapshot<K, V>
where
    K: Eq + Hash + Borrow<Rq>,
    Rq: ?Sized + Eq + Hash,
    V: Clone,
{
    type Output = V;

    #[inline]
    fn get(&self, req: &Rq) -> anyhow::Result<Self::Output> {
        self.data
            .get(req)
            .map(|r| r.data.clone())
            .ok_or_else(|| anyhow!("Key not found"))
    }
}

impl<K, V, Rq> CacheableSrc<Rq> for Snapshot<K, V>
where
    K: Eq + Hash + Borrow<Rq>,
    Rq: ?Sized + Eq + Hash,
    V: Clone,
{
    #[inline]
    fn etag(&self, req: &Rq) -> anyhow::Result<String> {
        self.data
            .get(req)
            .map(|r| r.etag.clone())
            .ok_or_else(|| anyhow!("Key not found"))
    }

    #[inline]
    fn get_with_etag(&self, req: &Rq) -> anyhow::Result<Response<Self::Output>> {
        self.data
            .get(req)
            .cloned()
            .ok_or_else(|| anyhow!("Key not found"))
    }

    #[inline]
    fn get_if_none_match(
        &self,
        req: &Rq,
        etag: &str,
    ) -> anyhow::Result<Option<Response<Self::Output>>> {
        match self.data.get(req) {
            Some(r) if r.etag == etag => Ok(None),
            Some(r) => Ok(Some(r.clone())),
            None => Err(anyhow!("Key not found")),
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::*;
    use _test_util::*;
    use rstest::rstest;
    use snapshot::Snapshot;

    #[test]
    fn test_with_datasrc() {
        let mut mock = MockSrc::with_call_count(&CallCount {
            get: Some(3),
            ..CallCount::zero()
        });
        let rqs = ["abc", "bcd", "cde"];

        let snap = Snapshot::with_datasrc(&mock, rqs).unwrap();

        for rq in rqs {
            let res = &snap.data.get(rq).as_ref().unwrap().data;
            assert_eq!(res, &MockSrc::to_res(rq).data);
        }
        assert_eq!(snap.data.len(), 3);
        mock.checkpoint();
    }

    #[test]
    fn test_with_datasrc_err() {
        let mut mock = MockSrc::with_call_count(&CallCount {
            get: Some(2),
            ..CallCount::zero()
        });
        let rqs = ["abc", ERR_REQ, "cde"];

        let snap = Snapshot::with_datasrc(&mock, rqs);

        assert_eq!(snap.unwrap_err().to_string(), ERR_MSG);
        mock.checkpoint();
    }

    #[test]
    fn test_with_cacheable() {
        let mut mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(3),
            ..CallCount::zero()
        });
        let rqs = ["abc", "bcd", "cde"];

        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        for rq in rqs {
            let res = snap.data.get(rq).unwrap();
            assert_eq!(res, &MockSrc::to_res(rq));
        }
        assert_eq!(snap.data.len(), 3);
        mock.checkpoint();
    }

    #[test]
    fn test_with_cacheable_err() {
        let mut mock = MockSrc::with_call_count(&CallCount {
            get_with_etag: Some(2),
            ..CallCount::zero()
        });
        let rqs = ["abc", ERR_REQ, "cde"];

        let snap = Snapshot::with_cacheable(&mock, rqs);

        assert_eq!(snap.unwrap_err().to_string(), ERR_MSG);
        mock.checkpoint();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize() {
        use serde_json::Value;
        let mock = MockSrc::with_call_count(&Default::default());
        let rqs = ["abc", "bcd", "cde"];
        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        let json = serde_json::to_value(snap).unwrap();

        let Value::Array(mut values) = json else {
            panic!("Expected an array, but got {:?}", json);
        };
        values.sort_by_key(|item| {
            item.as_object().unwrap()["key"]
                .as_str()
                .unwrap()
                .to_string()
        });
        let json = Value::Array(values);
        let expected = serde_json::json!(
            [
                {"key": "abc", "value": "cba", "etag": "etag-abc"},
                {"key": "bcd", "value": "dcb", "etag": "etag-bcd"},
                {"key": "cde", "value": "edc", "etag": "etag-cde"}
            ]
        );
        assert_eq!(json, expected);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deserialize() {
        let json = serde_json::json!(
            [
                {"key": "abc", "value": "cba", "etag": "etag-abc"},
                {"key": "bcd", "value": "dcb"},
                {"key": "cde", "value": "edc", "etag": "etag-cde"}
            ]
        );

        let snap: Snapshot<String, String> = serde_json::from_value(json).unwrap();

        assert_eq!(snap.data.len(), 3);
        assert_eq!(snap.data.get("abc").unwrap().data, "cba");
        assert_eq!(snap.data.get("abc").unwrap().etag, "etag-abc");
        assert_eq!(snap.data.get("bcd").unwrap().data, "dcb");
        assert_ne!(snap.data.get("bcd").unwrap().etag, "etag-bcd"); // generated by hash of key
        assert_eq!(snap.data.get("cde").unwrap().data, "edc");
        assert_eq!(snap.data.get("cde").unwrap().etag, "etag-cde");
    }

    #[rstest]
    #[case("abc")]
    #[case(ERR_REQ)]
    fn test_get(#[case] rq: &str) {
        let mock = MockSrc::with_call_count(&Default::default());
        let rqs = ["abc", "bcd", "cde"];
        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        let res = snap.get(rq);

        if rq == ERR_REQ {
            assert!(res.unwrap_err().to_string().contains("Key not found"));
        } else {
            assert_eq!(res.unwrap(), MockSrc::to_res(rq).data);
        }
    }

    #[rstest]
    #[case("abc")]
    #[case(ERR_REQ)]
    fn test_etag(#[case] rq: &str) {
        let mock = MockSrc::with_call_count(&Default::default());
        let rqs = ["abc", "bcd", "cde"];
        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        let etag = snap.etag(rq);

        if rq == ERR_REQ {
            assert!(etag.unwrap_err().to_string().contains("Key not found"));
        } else {
            assert_eq!(etag.unwrap(), MockSrc::to_etag(rq));
        }
    }

    #[rstest]
    #[case("abc")]
    #[case(ERR_REQ)]
    fn test_get_with_etag(#[case] rq: &str) {
        let mock = MockSrc::with_call_count(&Default::default());
        let rqs = ["abc", "bcd", "cde"];
        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        let res = snap.get_with_etag(rq);

        if rq == ERR_REQ {
            assert!(res.unwrap_err().to_string().contains("Key not found"));
        } else {
            assert_eq!(res.unwrap(), MockSrc::to_res(rq));
        }
    }

    #[rstest]
    #[case("abc", "etag-abc")]
    #[case("abc", "etag-xxx")]
    #[case(ERR_REQ, "etag-abc")]
    fn test_get_if_none_match(#[case] rq: &str, #[case] etag: &str) {
        let mock = MockSrc::with_call_count(&Default::default());
        let rqs = ["abc", "bcd", "cde"];
        let snap = Snapshot::with_cacheable(&mock, rqs).unwrap();

        let res = snap.get_if_none_match(rq, etag);

        match (rq, etag) {
            ("abc", "etag-abc") => assert_eq!(res.unwrap(), None),
            ("abc", "etag-xxx") => assert_eq!(res.unwrap().unwrap(), MockSrc::to_res("abc")),
            (ERR_REQ, "etag-abc") => {
                assert!(res.unwrap_err().to_string().contains("Key not found"))
            }
            _ => unreachable!(),
        }
    }
}
