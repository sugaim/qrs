use std::{cmp::Ordering, iter::FusedIterator};

use itertools::Itertools;

// -----------------------------------------------------------------------------
// Knot
//
/// An item of a series, which is a collection of values marked by grids(ordered keys)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
struct Knot<G, V> {
    pub grid: G,
    pub value: V,
}

// -----------------------------------------------------------------------------
// SeriesError
//
/// An error type for [`Series`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SeriesError {
    #[error("Lengths of grids and values must be the same. grids: {}, values: {}", .grids, .values)]
    LengthMismatch { grids: usize, values: usize },
    #[error("{}-th and the next grids are duplicated", .0)]
    DuplicatedGrid(usize),
    #[error("{}-th grid is greater than the next grid", .0)]
    UnorderedGrid(usize),
    #[error("{}-th and the next grids are not comparable", .0)]
    UncomparableGrid(usize),
}

// -----------------------------------------------------------------------------
// Series
//
/// A collection of values marked by grids(ordered keys)
///
/// As an iterable collection, it is similar to a map because items of both of them
/// are pairs of keys and values.
/// However, a sequential structure is emphasized in [`Series`] rather than
/// mapping from key to value.
/// For example, a time series is a typical example of [`Series`].
///
/// From a viewpoint of data structure, [`Series`] is column-oriented
/// and implemented with two vectors, one for grids and the other for values.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Series<G, V> {
    gs: Vec<G>,
    vs: Vec<V>,
}

//
// display, serde
//
#[cfg(feature = "serde")]
impl<G, V> serde::Serialize for Series<G, V>
where
    G: serde::Serialize,
    V: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.gs.len()))?;
        for (g, v) in self.gs.iter().zip(self.vs.iter()) {
            seq.serialize_element(&Knot { grid: g, value: v })?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, G, V> serde::Deserialize<'de> for Series<G, V>
where
    G: serde::Deserialize<'de> + PartialOrd,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Series<G, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut paired: Vec<Knot<G, V>> = Vec::deserialize(deserializer)?;
        paired.sort_by(|k1, k2| {
            k1.grid
                .partial_cmp(&k2.grid)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut gs = Vec::with_capacity(paired.len());
        let mut vs = Vec::with_capacity(paired.len());
        for Knot { grid, value } in paired {
            gs.push(grid);
            vs.push(value);
        }
        Self::new(gs, vs).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl<G, V> schemars::JsonSchema for Series<G, V>
where
    G: schemars::JsonSchema,
    V: schemars::JsonSchema,
{
    fn schema_name() -> String {
        format!("Series_for_{}_and_{}", G::schema_name(), V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!(
            "qrs_collections::Series<{}, {}>",
            G::schema_id(),
            V::schema_id()
        )
        .into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject};
        let mut schema = SchemaObject::default();
        schema.metadata().description = Some("Series for 1-dimensional interpolation".to_string());

        let array = schema.array();
        array.min_items = Some(2);
        array.additional_items = Some(Box::new(Schema::Bool(false)));
        array.items = Some(Knot::<G, V>::json_schema(gen).into());
        schema.into()
    }
}

//
// construction
//
impl<G: PartialOrd, V> Series<G, V> {
    /// Construct a new series from grids and values.
    ///
    /// # Errors
    /// - If the length of grids and values are different, it returns [`SeriesError::LengthMismatch`].
    /// - If the grids are not ordered, it returns [`SeriesError::UnorderedGrid`].
    /// - If the grids are not comparable, it returns [`SeriesError::UncomparableGrid`].
    /// - If the grids have duplicated elements, it returns [`SeriesError::DuplicatedGrid`].
    pub fn new(gs: Vec<G>, vs: Vec<V>) -> Result<Self, SeriesError> {
        if gs.len() != vs.len() {
            return Err(SeriesError::LengthMismatch {
                grids: gs.len(),
                values: vs.len(),
            });
        }
        for (i, (c, n)) in gs.iter().tuple_windows().enumerate() {
            match c.partial_cmp(n) {
                Some(Ordering::Less) => { /* OK */ }
                Some(Ordering::Equal) => return Err(SeriesError::DuplicatedGrid(i)),
                Some(Ordering::Greater) => return Err(SeriesError::UnorderedGrid(i)),
                None => return Err(SeriesError::UncomparableGrid(i)),
            }
        }
        Ok(Self { gs, vs })
    }
}

//
// methods
//
impl<G, V> Series<G, V> {
    /// Get the grids of the series.
    #[inline]
    pub fn grids(&self) -> &[G] {
        &self.gs
    }

    /// Get the values of the series.
    #[inline]
    pub fn values(&self) -> &[V] {
        &self.vs
    }

    /// Get the length of the series.
    #[inline]
    pub fn len(&self) -> usize {
        self.gs.len()
    }

    /// Check if the series is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.gs.is_empty()
    }

    /// Destruct the series into the pair of grids and values.
    /// This may be useful when you want to modify whole elements of the series.
    /// After destructing and modifying, you can reconstruct the series safely with [`Series::new`].
    #[inline]
    pub fn destruct(self) -> (Vec<G>, Vec<V>) {
        (self.gs, self.vs)
    }

    /// Get the iterator iterating over the pairs of grid and value.
    #[inline]
    pub fn iter(&self) -> SeriesIter<'_, G, V> {
        self.into_iter()
    }

    /// Get the mutable iterator iterating over the pairs of grid and value.
    /// Note that only the values can be modified.
    #[inline]
    pub fn iter_mut(&mut self) -> SeriesIterMut<'_, G, V> {
        self.into_iter()
    }

    /// Get the i-th element of the series.
    #[inline]
    pub fn get(&self, idx: usize) -> Option<(&G, &V)> {
        match (self.gs.get(idx), self.vs.get(idx)) {
            (Some(grid), Some(value)) => Some((grid, value)),
            _ => None,
        }
    }

    /// Get the mutable i-th element of the series.
    #[inline]
    pub fn get_mut(&mut self, idx: usize) -> Option<(&G, &mut V)> {
        match (self.gs.get(idx), self.vs.get_mut(idx)) {
            (Some(grid), Some(value)) => Some((grid, value)),
            _ => None,
        }
    }

    /// Insert an element into the series.
    /// If the grid is already in the series, the value is replaced with the given value
    /// and the old value is returned like [`std::collections::HashMap::insert`].
    ///
    /// Note that this method can return error because we use [`PartialOrd`] to compare the grids.
    /// If the given grid is not comparable with the grids in the series, it returns an error.
    ///
    /// Also note that [`Series`] is implemented with [Vec].
    /// So, the time complexity of this method is O(n).
    ///
    /// # Example
    /// ```
    /// use qrs_collections::Series;
    ///
    /// let grids: Vec<f64> = vec![0.0, 10.0, 20.0];
    /// let values: Vec<i32> = vec![0, 1, 2];
    ///
    /// let mut series = Series::new(grids, values).unwrap();
    ///
    /// let _ = series.insert(5.0, 3).unwrap();
    /// assert_eq!(series.grids(), &[0.0, 5.0, 10.0, 20.0]);
    /// assert_eq!(series.values(), &[0, 3, 1, 2]);
    ///
    /// // non-comparable
    /// let res = series.insert(f64::NAN, 4);
    /// assert!(res.is_err());
    /// ```
    pub fn insert(&mut self, grid: G, value: V) -> Result<Option<V>, SeriesError>
    where
        G: PartialOrd,
    {
        let idx = self.gs.partition_point(|g| g < &grid);
        if self.is_empty() {
            self.gs.push(grid);
            self.vs.push(value);
            return Ok(None);
        }
        let uncomparable_at = |i: usize| {
            self.gs
                .get(i)
                .map(|g| g.partial_cmp(&grid).is_none())
                .unwrap_or(false)
        };
        if uncomparable_at(idx) || uncomparable_at(idx + 1) {
            return Err(SeriesError::UncomparableGrid(idx));
        }
        if self.gs.get(idx) == Some(&grid) {
            return Ok(Some(std::mem::replace(&mut self.vs[idx], value)));
        }
        self.gs.insert(idx, grid);
        self.vs.insert(idx, value);
        Ok(None)
    }

    /// Remove the i-th element of the series.
    /// If the index is out of range, it returns [`None`].
    #[inline]
    pub fn remove(&mut self, idx: usize) -> Option<(G, V)> {
        if idx < self.len() {
            Some((self.gs.remove(idx), self.vs.remove(idx)))
        } else {
            None
        }
    }

    /// Get the index of interval (left: close, right: open) that contains the given point.
    /// That is, the index of the interval `[gs[i], gs[i+1])` where `gs` is the grids of the series.
    ///
    /// ```txt
    /// -x--[0]-----[1]-----[2]---- => 0
    /// ----[0]=x---[1]-----[2]---- => 0
    /// ----[0]--x--[1]-----[2]---- => 0
    /// ----[0]-----[1]=x---[2]---- => 1
    /// ----[0]-----[1]--x--[2]---- => 1
    /// ----[0]-----[1]-----[2]=x-- => 1
    /// ----[0]-----[1]-----[2]--x- => 1
    /// ```
    ///
    /// Returns [`None`] if the series does not have an interval.
    /// That is, the length of the series is less than 2.
    ///
    /// # Examples
    /// ```
    /// use qrs_collections::Series;
    ///
    /// let Series = Series::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();
    ///
    /// assert_eq!(Series.interval_index_of(&-1).unwrap(), 0);
    /// assert_eq!(Series.interval_index_of(&0).unwrap(), 0);
    /// assert_eq!(Series.interval_index_of(&5).unwrap(), 0);
    /// assert_eq!(Series.interval_index_of(&10).unwrap(), 1);
    /// assert_eq!(Series.interval_index_of(&15).unwrap(), 1);
    /// assert_eq!(Series.interval_index_of(&20).unwrap(), 1);
    /// assert_eq!(Series.interval_index_of(&25).unwrap(), 1);
    /// ```
    ///
    pub fn interval_index_of(&self, x: &G) -> Option<usize>
    where
        G: PartialOrd,
    {
        if self.gs.len() < 2 {
            return None;
        }
        let idx = self.gs[0..self.gs.len() - 1].partition_point(|g| g <= x);
        Some(idx.max(1) - 1)
    }
}

impl<G, V> IntoIterator for Series<G, V> {
    type Item = (G, V);
    type IntoIter = SeriesIntoIter<G, V>;

    fn into_iter(self) -> Self::IntoIter {
        SeriesIntoIter {
            gs: self.gs.into_iter(),
            vs: self.vs.into_iter(),
        }
    }
}

impl<'a, G, V> IntoIterator for &'a Series<G, V> {
    type Item = (&'a G, &'a V);
    type IntoIter = SeriesIter<'a, G, V>;

    fn into_iter(self) -> Self::IntoIter {
        SeriesIter {
            gs: self.gs.iter(),
            vs: self.vs.iter(),
        }
    }
}

impl<'a, G, V> IntoIterator for &'a mut Series<G, V> {
    type Item = (&'a G, &'a mut V);
    type IntoIter = SeriesIterMut<'a, G, V>;

    fn into_iter(self) -> Self::IntoIter {
        SeriesIterMut {
            gs: self.gs.iter(),
            vs: self.vs.iter_mut(),
        }
    }
}

// -----------------------------------------------------------------------------
// SeriesIter
//
pub struct SeriesIter<'a, G, V> {
    gs: std::slice::Iter<'a, G>,
    vs: std::slice::Iter<'a, V>,
}

impl<'a, G, V> Iterator for SeriesIter<'a, G, V> {
    type Item = (&'a G, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.gs.next(), self.vs.next()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}

impl<'a, G, V> ExactSizeIterator for SeriesIter<'a, G, V> {
    fn len(&self) -> usize {
        self.gs.len()
    }
}

impl<'a, G, V> DoubleEndedIterator for SeriesIter<'a, G, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.gs.next_back(), self.vs.next_back()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}
impl<'a, G, V> FusedIterator for SeriesIter<'a, G, V> {}

// -----------------------------------------------------------------------------
// SeriesIntoIter
//
pub struct SeriesIntoIter<G, V> {
    gs: std::vec::IntoIter<G>,
    vs: std::vec::IntoIter<V>,
}

impl<G, V> Iterator for SeriesIntoIter<G, V> {
    type Item = (G, V);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.gs.next(), self.vs.next()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}

impl<G, V> ExactSizeIterator for SeriesIntoIter<G, V> {
    fn len(&self) -> usize {
        self.gs.len()
    }
}

impl<G, V> DoubleEndedIterator for SeriesIntoIter<G, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.gs.next_back(), self.vs.next_back()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}

impl<G, V> FusedIterator for SeriesIntoIter<G, V> {}

// -----------------------------------------------------------------------------
// SeriesIterMut
//
pub struct SeriesIterMut<'a, G, V> {
    gs: std::slice::Iter<'a, G>,
    vs: std::slice::IterMut<'a, V>,
}

impl<'a, G, V> Iterator for SeriesIterMut<'a, G, V> {
    type Item = (&'a G, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.gs.next(), self.vs.next()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}

impl<'a, G, V> ExactSizeIterator for SeriesIterMut<'a, G, V> {
    fn len(&self) -> usize {
        self.gs.len()
    }
}

impl<'a, G, V> DoubleEndedIterator for SeriesIterMut<'a, G, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.gs.next_back(), self.vs.next_back()) {
            (Some(g), Some(v)) => Some((g, v)),
            _ => None,
        }
    }
}

impl<'a, G, V> FusedIterator for SeriesIterMut<'a, G, V> {}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn assert_valid_data<G: PartialOrd, V>(s: &Series<G, V>) {
        assert_eq!(s.gs.len(), s.vs.len());
        for (i, (c, n)) in s.gs.iter().zip(s.gs.iter().skip(1)).enumerate() {
            assert!(c < n, "unorderd: {}-th and the next grids", i);
        }
    }

    #[cfg(feature = "serde")]
    #[rstest::rstest]
    #[case(Series::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap(), serde_json::json!([{"grid": 0, "value": 0}, {"grid": 10, "value": 1}, {"grid": 20, "value": 2}]))]
    #[case(Series::new(vec![0], vec![1]).unwrap(), serde_json::json!([{"grid": 0, "value": 1}]))]
    #[case(Series::new(vec![], vec![]).unwrap(), serde_json::json!([]))]
    fn test_serialize(#[case] input: Series<u32, u32>, #[case] expected: serde_json::Value) {
        let serialized = serde_json::to_value(input).unwrap();

        assert_eq!(serialized, expected);
    }

    #[cfg(feature = "serde")]
    #[rstest::rstest]
    #[case(serde_json::json!([{"grid": 0, "value": 1}, {"grid": 1, "value": 2}]), Ok(Series::new(vec![0, 1], vec![1, 2]).unwrap()))]
    #[case(serde_json::json!([{"grid": 1, "value": 2}, {"grid": 0, "value": 1}]), Ok(Series::new(vec![0, 1], vec![1, 2]).unwrap()))]
    #[case(serde_json::json!([{"grid": 0, "value": 1}, {"grid": 0, "value": 2}]), Err("0-th and the next grids are duplicated".to_string()))]
    fn test_deserialize(
        #[case] json: serde_json::Value,
        #[case] expected: Result<Series<u32, u32>, String>,
    ) {
        let deserialized: Result<Series<u32, u32>, _> =
            serde_json::from_value(json).map_err(|e| e.to_string());

        assert_eq!(deserialized, expected);
        if let Ok(series) = &deserialized {
            assert_valid_data(series);
        }
    }

    #[rstest]
    #[case(vec![0., 10., 20.], vec![0, 1, 2], None)]
    #[case(vec![0.], vec![1], None)]
    #[case(vec![], vec![], None)]
    #[case(vec![0., 10., 20.], vec![0, 1], Some(SeriesError::LengthMismatch { grids: 3, values: 2 }))]
    #[case(vec![0., 0., 20.], vec![0, 1, 2], Some(SeriesError::DuplicatedGrid(0)))]
    #[case(vec![0., 10., 5.], vec![0, 1, 2], Some(SeriesError::UnorderedGrid(1)))]
    #[case(vec![0., f32::NAN, 20.], vec![0, 1, 2], Some(SeriesError::UncomparableGrid(0)))]
    fn test_new(
        #[case] grids: Vec<f32>,
        #[case] values: Vec<i32>,
        #[case] err: Option<SeriesError>,
    ) {
        let res = Series::new(grids.clone(), values.clone());

        match err {
            Some(e) => assert_eq!(res, Err(e)),
            None => {
                let series = res.unwrap();
                assert_valid_data(&series);
                assert_eq!(series.grids(), grids.as_slice());
                assert_eq!(series.values(), values.as_slice());
            }
        }
    }

    #[rstest]
    #[case(-1., Some(0), Ok(None))]
    #[case(0., None, Ok(Some(0)))]
    #[case(5., Some(1), Ok(None))]
    #[case(10., None, Ok(Some(1)))]
    #[case(15., Some(2), Ok(None))]
    #[case(20., None, Ok(Some(2)))]
    #[case(25., Some(3), Ok(None))]
    #[case(f32::NAN, None, Err(SeriesError::UncomparableGrid(0)))]
    fn test_insert(
        #[case] grid: f32,
        #[case] inserted_at: Option<usize>,
        #[case] output: Result<Option<i32>, SeriesError>,
    ) {
        let mut gs = vec![0., 10., 20.];
        let mut vs = vec![0, 1, 2];
        let new_val = 42;
        let mut series = Series::new(gs.clone(), vs.clone()).unwrap();

        let res = series.insert(grid, new_val);

        assert_eq!(res, output);
        let (gs, vs) = if let Some(at) = inserted_at {
            gs.insert(at, grid);
            vs.insert(at, new_val);
            (gs, vs)
        } else if output.is_ok() {
            let idx = gs.as_slice().partition_point(|g| g < &grid);
            vs[idx] = new_val;
            (gs, vs)
        } else {
            (gs, vs)
        };
        assert_valid_data(&series);
        assert_eq!(series.grids(), gs.as_slice());
        assert_eq!(series.values(), vs.as_slice());
        assert_eq!(res, output);
    }

    #[rstest]
    #[case(-1, 0)]
    #[case(0, 0)]
    #[case(5, 0)]
    #[case(10, 1)]
    #[case(15, 1)]
    #[case(20, 1)]
    #[case(25, 1)]
    fn test_interval_index_of(#[case] grid: i32, #[case] expected: usize) {
        let series = Series::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();

        let res = series.interval_index_of(&grid).unwrap();

        assert_eq!(res, expected);
    }
}
