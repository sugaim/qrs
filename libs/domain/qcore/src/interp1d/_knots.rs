use anyhow::ensure;
use chrono::NaiveDate;
use schemars::{
    schema::{Schema, SchemaObject},
    JsonSchema,
};
use serde::{ser::SerializeSeq, Serialize};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub(super) struct Knots<G, V> {
    gs: Vec<G>,
    vs: Vec<V>,
}

//
// display, serde
//
impl<G, V> Serialize for Knots<G, V>
where
    G: Serialize,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.gs.len()))?;
        for (g, v) in self.gs.iter().zip(self.vs.iter()) {
            seq.serialize_element(&(g, v))?;
        }
        seq.end()
    }
}

impl<'de, G, V> serde::Deserialize<'de> for Knots<G, V>
where
    G: serde::Deserialize<'de> + PartialOrd,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Knots<G, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut paired: Vec<(G, V)> = Vec::deserialize(deserializer)?;
        paired.sort_by(|(g1, _), (g2, _)| g1.partial_cmp(g2).unwrap_or(std::cmp::Ordering::Equal));
        let (gs, vs): (Vec<G>, Vec<V>) = paired.into_iter().unzip();
        Self::new(gs, vs).map_err(serde::de::Error::custom)
    }
}

impl<G, V> JsonSchema for Knots<G, V>
where
    G: JsonSchema,
    V: JsonSchema,
{
    fn schema_name() -> String {
        format!("Knots_{}_{}", G::schema_name(), V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!(
            "qcore::interp1d::Knots<{}, {}>",
            G::schema_id(),
            V::schema_id()
        )
        .into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = SchemaObject::default();
        schema.metadata().description = Some("Knots for 1-dimensional interpolation".to_string());
        schema.metadata().title = Some(Self::schema_name());
        schema.metadata().id = Some(Self::schema_id().into_owned());
        schema.metadata().examples = vec![serde_json::json!([
            [NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(), 0.05],
            [NaiveDate::from_ymd_opt(2021, 1, 7).unwrap(), 0.1],
            [NaiveDate::from_ymd_opt(2021, 2, 1).unwrap(), 0.05],
        ])];
        schema.metadata().deprecated = false;

        let array = schema.array();
        array.min_items = Some(2);
        array.additional_items = Some(Box::new(Schema::Bool(false)));
        array.items = Some(<(G, V)>::json_schema(gen).into());
        schema.into()
    }
}

//
// construction
//
impl<G: PartialOrd, V> Knots<G, V> {
    pub fn new(gs: Vec<G>, vs: Vec<V>) -> Result<Self, anyhow::Error> {
        ensure!(
            gs.len() == vs.len(),
            "Lengths of grids and values must be the same"
        );
        ensure!(2 <= gs.len(), "At least two knots are required");
        ensure!(
            gs.windows(2).all(|w| w[0] < w[1]),
            "Grids must be sorted in ascending order"
        );
        Ok(Self { gs, vs })
    }
}

//
// methods
//
impl<G: PartialOrd, V> Knots<G, V> {
    #[inline]
    pub fn grids(&self) -> &[G] {
        &self.gs
    }

    #[inline]
    pub fn values(&self) -> &[V] {
        &self.vs
    }

    #[inline]
    pub fn destruct(self) -> (Vec<G>, Vec<V>) {
        (self.gs, self.vs)
    }

    #[inline]
    pub fn force_get(&self, idx: usize) -> (&G, &V) {
        (&self.gs[idx], &self.vs[idx])
    }

    /// Get the index of interval (left: close, right: open) that contains the given point.
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
    /// # Examples
    /// ```ignore
    /// use qcore::interp1d::lerp::Knots;
    ///
    /// let knots = Knots::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();
    ///
    /// assert_eq!(knots.interval_index_of(&-1), 0);
    /// assert_eq!(knots.interval_index_of(&0), 0);
    /// assert_eq!(knots.interval_index_of(&5), 0);
    /// assert_eq!(knots.interval_index_of(&10), 1);
    /// assert_eq!(knots.interval_index_of(&15), 1);
    /// assert_eq!(knots.interval_index_of(&20), 1);
    /// assert_eq!(knots.interval_index_of(&25), 1);
    /// ```
    ///
    pub fn interval_index_of(&self, x: &G) -> usize {
        debug_assert!(2 <= self.gs.len(), "ctor must guarantee at least two knots");
        let idx = self.gs[0..self.gs.len() - 1].partition_point(|g| g <= x);
        if idx == 0 {
            0
        } else {
            idx - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let knots = Knots::new(
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 7).unwrap(),
                NaiveDate::from_ymd_opt(2021, 2, 1).unwrap(),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let serialized = serde_json::to_string(&knots).unwrap();
        assert_eq!(
            serialized,
            r#"[["2021-01-01",0],["2021-01-07",1],["2021-02-01",2]]"#
        );
    }

    #[test]
    fn test_deserialize() {
        let serialized = r#"[["2021-01-01",0],["2021-01-07",1],["2021-02-01",2]]"#;
        let deserialized: Knots<NaiveDate, i32> = serde_json::from_str(serialized).unwrap();
        let knots = Knots::new(
            vec![
                NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2021, 1, 7).unwrap(),
                NaiveDate::from_ymd_opt(2021, 2, 1).unwrap(),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(deserialized, knots);

        // unordered: unordered elements are sorted in deserialization
        let serialized = r#"[["2021-01-07",1],["2021-01-01",0],["2021-02-01",2]]"#;
        let deserialized: Result<Knots<NaiveDate, i32>, _> = serde_json::from_str(serialized);
        assert_eq!(deserialized.unwrap(), knots);

        // error
        let serialized = r#"[["2021-01-07",1],["2021-01-01",0],["2021-01-01",2]]"#;
        let deserialized: Result<Knots<NaiveDate, i32>, _> = serde_json::from_str(serialized);
        assert!(deserialized.is_err());

        let serialized = r#"[["2021-01-07",1]]"#;
        let deserialized: Result<Knots<NaiveDate, i32>, _> = serde_json::from_str(serialized);
        assert!(deserialized.is_err());
    }

    #[test]
    fn test_interval_index_of() {
        let knots = Knots::new(vec![0, 10, 20], vec![0, 1, 2]).unwrap();
        assert_eq!(knots.interval_index_of(&-1), 0);
        assert_eq!(knots.interval_index_of(&0), 0);
        assert_eq!(knots.interval_index_of(&5), 0);
        assert_eq!(knots.interval_index_of(&10), 1);
        assert_eq!(knots.interval_index_of(&15), 1);
        assert_eq!(knots.interval_index_of(&20), 1);
        assert_eq!(knots.interval_index_of(&25), 1);
    }
}
