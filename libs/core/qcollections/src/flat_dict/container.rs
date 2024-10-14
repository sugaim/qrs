use std::{borrow::Borrow, cmp::Ordering};

use itertools::Itertools;

use super::Error;

// -----------------------------------------------------------------------------
// FlatDict
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FlatDict<K, V> {
    ks: Vec<K>,
    vs: Vec<V>,
}

//
// serde
//
impl<K, V> serde::Serialize for FlatDict<K, V>
where
    K: serde::Serialize,
    V: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Item<'a, K, V> {
            key: &'a K,
            value: &'a V,
        }
        let kvs = self.ks.iter().zip(self.vs.iter());
        let items: Vec<_> = kvs.map(|(k, v)| Item { key: k, value: v }).collect();
        items.serialize(serializer)
    }
}

impl<'de, K, V> serde::Deserialize<'de> for FlatDict<K, V>
where
    K: serde::Deserialize<'de> + PartialOrd,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Item<K, V> {
            key: K,
            value: V,
        }
        let mut items: Vec<Item<K, V>> = serde::Deserialize::deserialize(deserializer)?;
        items.sort_by(|a, b| a.key.partial_cmp(&b.key).unwrap_or(Ordering::Equal));
        let (ks, vs) = items.into_iter().map(|item| (item.key, item.value)).unzip();
        Self::with_sorted(ks, vs).map_err(serde::de::Error::custom)
    }
}

impl<K, V> schemars::JsonSchema for FlatDict<K, V>
where
    K: schemars::JsonSchema,
    V: schemars::JsonSchema,
{
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!(
            "qcollections::FlatMap<{}, {}>",
            K::schema_name(),
            V::schema_name()
        )
        .into()
    }
    fn schema_name() -> String {
        format!("FlatMap_for_{}_and_{}", K::schema_name(), V::schema_name())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        #[derive(schemars::JsonSchema)]
        #[allow(unused)]
        struct Item<K, V> {
            key: K,
            value: V,
        }

        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Array.into()),
            array: Some(Box::new(schemars::schema::ArrayValidation {
                items: Some(schemars::schema::SingleOrVec::Single(Box::new(
                    Item::<K, V>::json_schema(gen),
                ))),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

//
// ctor
//
impl<K, V> FlatDict<K, V> {
    #[inline]
    pub fn with_sorted(ks: Vec<K>, vs: Vec<V>) -> Result<Self, Error>
    where
        K: PartialOrd,
    {
        if ks.len() != vs.len() {
            return Err(Error::SizeMismatch {
                keys: ks.len(),
                values: vs.len(),
            });
        }
        for cmp in ks.iter().tuple_windows().map(|(a, b)| a.partial_cmp(b)) {
            match cmp {
                None => return Err(Error::Unsortable),
                Some(Ordering::Greater) => return Err(Error::Unordered),
                Some(Ordering::Equal) => return Err(Error::Duplicated),
                _ => (),
            }
        }
        Ok(FlatDict { ks, vs })
    }

    #[inline]
    pub fn with_data(ks: Vec<K>, vs: Vec<V>) -> Result<Self, Error>
    where
        K: PartialOrd,
    {
        if ks.len() != vs.len() {
            return Err(Error::SizeMismatch {
                keys: ks.len(),
                values: vs.len(),
            });
        }
        let mut paired = ks.into_iter().zip(vs).collect::<Vec<_>>();
        paired.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        let (ks, vs) = paired.into_iter().unzip();
        Self::with_sorted(ks, vs)
    }
}

//
// methods
//
impl<K, V> IntoIterator for FlatDict<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.ks.into_iter().zip(self.vs)
    }
}

impl<'a, K, V> IntoIterator for &'a FlatDict<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.ks.iter().zip(self.vs.iter())
    }
}

impl<K, V> FlatDict<K, V> {
    #[inline]
    pub fn keys(&self) -> &[K] {
        &self.ks
    }

    #[inline]
    pub fn values(&self) -> &[V] {
        &self.vs
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.ks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ks.is_empty()
    }

    #[inline]
    pub fn at(&self, idx: usize) -> Option<(&K, &V)> {
        self.ks
            .get(idx)
            .and_then(|k| self.vs.get(idx).map(|v| (k, v)))
    }

    #[inline]
    pub fn at_mut(&mut self, idx: usize) -> Option<(&K, &mut V)> {
        self.ks
            .get(idx)
            .and_then(|k| self.vs.get_mut(idx).map(|v| (k, v)))
    }

    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        let idx = self.ks.binary_search_by(|k| (*k.borrow()).cmp(key)).ok()?;
        self.vs.get(idx)
    }

    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord,
    {
        let idx = self.ks.binary_search_by(|k| (*k.borrow()).cmp(key)).ok()?;
        self.vs.get_mut(idx)
    }

    /// Get an interval index which the key belongs to.
    ///
    /// More precisely, this returns the index `i` which satisfies either of the following:
    /// * `i == 0` and `key < ks[0]`
    /// * `i == len() - 2` and `ks[len() - 2] <= key`
    /// * `ks[i] <= key < ks[i + 1]`
    ///
    /// Conversely, this returns `None` in the following cases:
    /// * `len() < 2` because 'interval' does not make sense.
    /// * all of above conditions are not satisfied (due to unorderable value)
    #[inline]
    pub fn interval_index<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: PartialOrd,
    {
        if self.len() < 2 {
            return None;
        }
        let candidate = self.ks[..(self.len() - 1)]
            .partition_point(|k| k.borrow() <= key)
            .saturating_sub(1);

        if (candidate == 0 && key < self.ks[0].borrow())
            || (candidate == self.len() - 2 && self.ks[candidate].borrow() <= key)
            || (self.ks[candidate].borrow() <= key && key < self.ks[candidate + 1].borrow())
        {
            Some(candidate)
        } else {
            None
        }
    }

    #[inline]
    pub fn destruct(self) -> (Vec<K>, Vec<V>) {
        (self.ks, self.vs)
    }
}

#[cfg(test)]
mod tests {
    use core::f64;

    use rstest::rstest;

    use super::*;

    #[test]
    fn test_with_sorted_ok() {
        let ks = vec![1, 2, 3];
        let vs = vec!["a", "b", "c"];

        let map = FlatDict::with_sorted(ks.clone(), vs.clone()).unwrap();

        assert_eq!(map.keys(), ks.as_slice());
        assert_eq!(map.values(), vs.as_slice());
        assert_eq!(map.len(), 3);
        assert!(!map.is_empty());
    }

    #[test]
    fn test_with_sorted_ok_empty() {
        let ks: Vec<i64> = vec![];
        let vs: Vec<&'static str> = vec![];

        let map = FlatDict::with_sorted(ks.clone(), vs.clone()).unwrap();

        assert_eq!(map.keys(), ks.as_slice());
        assert_eq!(map.values(), vs.as_slice());
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_with_sorted_err_size() {
        let ks = vec![1, 2];
        let vs = vec!["a", "b", "c"];

        let err = FlatDict::with_sorted(ks, vs).unwrap_err();

        assert!(matches!(err, Error::SizeMismatch { keys: 2, values: 3 }));
    }

    #[test]
    fn test_with_sorted_err_unsortable() {
        let ks = vec![1f64, f64::NAN, 3f64];
        let vs = vec!["a", "b", "c"];

        let err = FlatDict::with_sorted(ks, vs).unwrap_err();

        assert!(matches!(err, Error::Unsortable));
    }

    #[test]
    fn test_with_sorted_err_unordered() {
        let ks = vec![1, 3, 2];
        let vs = vec!["a", "b", "c"];

        let err = FlatDict::with_sorted(ks, vs).unwrap_err();

        assert!(matches!(err, Error::Unordered));
    }

    #[test]
    fn test_with_sorted_err_duplicated() {
        let ks = vec![1, 2, 2, 3];
        let vs = vec!["a", "b", "c", "d"];

        let err = FlatDict::with_sorted(ks, vs).unwrap_err();

        assert!(matches!(err, Error::Duplicated));
    }

    #[test]
    fn test_with_data_ok() {
        let ks = vec![1, 3, 2];
        let vs = vec!["a", "c", "b"];

        let map = FlatDict::with_data(ks, vs).unwrap();

        assert_eq!(map.keys(), &[1, 2, 3]);
        assert_eq!(map.values(), &["a", "b", "c"]);
        assert_eq!(map.len(), 3);
        assert!(!map.is_empty());
    }

    #[test]
    fn test_with_data_ok_empty() {
        let ks: Vec<i64> = vec![];
        let vs: Vec<&'static str> = vec![];

        let map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        assert_eq!(map.keys(), ks.as_slice());
        assert_eq!(map.values(), vs.as_slice());
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_with_data_err_size() {
        let ks = vec![1, 2];
        let vs = vec!["a", "b", "c"];

        let err = FlatDict::with_data(ks, vs).unwrap_err();

        assert!(matches!(err, Error::SizeMismatch { keys: 2, values: 3 }));
    }

    #[test]
    fn test_with_data_err_unsortable() {
        let ks = vec![1f64, f64::NAN, 3f64];
        let vs = vec!["a", "b", "c"];

        let err = FlatDict::with_data(ks, vs).unwrap_err();

        assert!(matches!(err, Error::Unsortable));
    }

    #[test]
    fn test_serialize() {
        let ks = vec![1, 3, 2];
        let vs = vec!["a", "c", "b"];
        let map = FlatDict::with_data(ks, vs).unwrap();

        let json = serde_json::to_value(map).unwrap();
        let expected = serde_json::json!([
            {"key": 1, "value": "a"},
            {"key": 2, "value": "b"},
            {"key": 3, "value": "c"},
        ]);
        assert_eq!(json, expected);
    }

    #[test]
    fn test_serialize_empty() {
        let ks: Vec<i64> = vec![];
        let vs: Vec<&'static str> = vec![];
        let map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        let json = serde_json::to_value(map).unwrap();
        let expected = serde_json::json!([]);
        assert_eq!(json, expected);
    }

    #[test]
    fn test_deserizlize() {
        let json = serde_json::json!([
            {"key": 2, "value": "b"},
            {"key": 1, "value": "a"},
            {"key": 3, "value": "c"},
        ]);

        let map: FlatDict<i64, String> = serde_json::from_value(json).unwrap();

        assert_eq!(map.keys(), &[1, 2, 3]);
        assert_eq!(map.values(), &["a", "b", "c"]);
    }

    #[test]
    fn test_deserizlize_empty() {
        let json = serde_json::json!([]);

        let map: FlatDict<i64, String> = serde_json::from_value(json).unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn test_deserialize_err() {
        let json = serde_json::json!([
            {"key": 1, "value": "a"},
            {"key": 2, "value": "c"},
            {"key": 2, "value": "b"},
        ]);

        let res = serde_json::from_value::<FlatDict<i64, String>>(json);

        assert!(res.is_err());
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(0, 2)]
    #[case(2, 0)]
    #[case(2, 1)]
    #[case(2, 2)]
    #[case(2, 3)]
    fn test_at(#[case] size: usize, #[case] at: usize) {
        let ks = (0..size).collect::<Vec<_>>();
        let vs = (0..size).map(|i| i.to_string()).collect::<Vec<_>>();
        let map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        let res = map.at(at);

        if at < size {
            assert_eq!(res, Some((&at, &at.to_string())));
        } else {
            assert_eq!(res, None);
        }
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(0, 2)]
    #[case(2, 0)]
    #[case(2, 1)]
    #[case(2, 2)]
    #[case(2, 3)]
    fn test_at_mut(#[case] size: usize, #[case] at: usize) {
        let ks = (0..size).collect::<Vec<_>>();
        let vs = (0..size).map(|i| i.to_string()).collect::<Vec<_>>();
        let mut map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        if at < size {
            {
                let res = map.at_mut(at);
                assert_eq!(res, Some((&at, &mut at.to_string())));
                res.unwrap().1.push('!');
            }
            assert_eq!(map.at(at), Some((&at, &(at.to_string() + "!"))));
        } else {
            assert_eq!(map.at_mut(at), None);
        }
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(0, 2)]
    #[case(2, 0)]
    #[case(2, 1)]
    #[case(2, 2)]
    #[case(2, 3)]
    fn test_get(#[case] size: usize, #[case] key: usize) {
        let ks = (0..size).collect::<Vec<_>>();
        let vs = (0..size).map(|i| i.to_string()).collect::<Vec<_>>();
        let map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        let res = map.get(&key);

        if key < size {
            assert_eq!(res, Some(&key.to_string()));
        } else {
            assert_eq!(res, None);
        }
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(0, 2)]
    #[case(2, 0)]
    #[case(2, 1)]
    #[case(2, 2)]
    #[case(2, 3)]
    fn test_get_mut(#[case] size: usize, #[case] key: usize) {
        let ks = (0..size).collect::<Vec<_>>();
        let vs = (0..size).map(|i| i.to_string()).collect::<Vec<_>>();
        let mut map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        if key < size {
            {
                let res = map.get_mut(&key);
                assert_eq!(res, Some(&mut key.to_string()));
                res.unwrap().push('!');
            }
            assert_eq!(map.get(&key), Some(&(key.to_string() + "!")));
        } else {
            assert_eq!(map.get_mut(&key), None);
        }
    }

    #[rstest]
    #[case(f64::NEG_INFINITY, Some(0))]
    #[case(0., Some(0))]
    #[case(1., Some(0))]
    #[case(1.5, Some(0))]
    #[case(2., Some(1))]
    #[case(3., Some(1))]
    #[case(3.5, Some(1))]
    #[case(4., Some(2))]
    #[case(5., Some(2))]
    #[case(5.5, Some(2))]
    #[case(f64::INFINITY, Some(2))]
    #[case(f64::NAN, None)]
    fn test_interval_index(#[case] x: f64, #[case] expected: Option<usize>) {
        let ks = vec![1.0, 2.0, 4.0, 5.0];
        let vs = vec!["a", "b", "d", "e"];
        let map = FlatDict::with_data(ks.clone(), vs.clone()).unwrap();

        let res = map.interval_index(&x);

        assert_eq!(res, expected);
    }

    #[rstest]
    #[case(f64::NEG_INFINITY)]
    #[case(0.)]
    #[case(1.)]
    #[case(1.5)]
    #[case(2.)]
    #[case(3.)]
    #[case(3.5)]
    #[case(4.)]
    #[case(5.)]
    #[case(5.5)]
    #[case(f64::INFINITY)]
    #[case(f64::NAN)]
    fn test_interval_index_err(#[case] x: f64) {
        let empty = FlatDict::<f64, &str>::with_data(vec![], vec![]).unwrap();
        let single = FlatDict::with_data(vec![1.0], vec!["a"]).unwrap();

        assert_eq!(empty.interval_index(&x), None);
        assert_eq!(single.interval_index(&x), None);
    }
}
