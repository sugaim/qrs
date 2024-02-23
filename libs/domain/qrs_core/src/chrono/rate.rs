use std::collections::BTreeMap;

use maplit::btreeset;
use num::Zero;
use schemars::{schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::num::{Arithmetic, FloatBased, Vector};

use super::Duration;

// -----------------------------------------------------------------------------
// _Rate
//
#[derive(Debug, Clone, Copy)]
enum _Rate<V> {
    /// WHen the duration is zero
    Diverged {
        chg: V, // original change
    },
    /// When the duration is not zero
    /// In this case, the change is normalized to the change per unit time
    Finite {
        chg: V, // change per duration
        dur: Duration,
    },
}

#[inline]
fn _unit_time() -> Duration {
    Duration::with_secs(1)
}

#[inline]
fn _dur_to_num_units<T: FloatBased>(dur: Duration) -> T::BaseFloat {
    let sec = T::nearest_base_float_of(dur.secs() as _);
    let subsec = T::nearest_base_float_of(dur.subsec_nanos() as f64 * 1e-9);
    sec + subsec
}

// -----------------------------------------------------------------------------
// Rate
//

/// A Rate, which is a change per given duration.
#[derive(Debug, Clone, Copy)]
pub struct Rate<V>(_Rate<V>);

//
// display, serde
//

impl<V: FloatBased + Vector<V::BaseFloat> + Serialize> Serialize for Rate<V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let orig_chg = match &self.0 {
            _Rate::Diverged { chg } => chg.clone(),
            _Rate::Finite { chg, dur } => chg.clone() * &_dur_to_num_units::<V>(*dur),
        };
        #[derive(Serialize)]
        struct VelocityHelper<V> {
            change: V,
            duration: Duration,
        }

        VelocityHelper {
            change: orig_chg,
            duration: match self.0 {
                _Rate::Diverged { .. } => Duration::zero(),
                _Rate::Finite { dur, .. } => dur,
            },
        }
        .serialize(serializer)
    }
}

impl<'de, V: FloatBased + Vector<V::BaseFloat> + Deserialize<'de>> Deserialize<'de> for Rate<V> {
    fn deserialize<D>(deserializer: D) -> Result<Rate<V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VelocityHelper<V> {
            change: V,
            duration: Duration,
        }
        let helper = VelocityHelper::deserialize(deserializer)?;
        Ok(Rate::new(helper.change, helper.duration))
    }
}

impl<V: JsonSchema> JsonSchema for Rate<V> {
    fn schema_name() -> String {
        format!("Rate_for_{}", V::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_core::chrono::Rate<{}>", V::schema_id()).into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            ..Default::default()
        };
        res.metadata().description = Some("A change per given duration.".to_string());
        res.object().properties = {
            let mut props = BTreeMap::default();

            let mut chg = gen.subschema_for::<V>();
            if let Schema::Object(ref mut chg) = chg {
                chg.metadata().description = Some("Change in the given duration".to_string());
            }
            props.insert("change".to_owned(), gen.subschema_for::<V>());

            let mut dur = gen.subschema_for::<Duration>();
            if let Schema::Object(ref mut dur) = dur {
                dur.metadata().description = Some("Duration of the change".to_string());
            }
            props.insert("duration".to_owned(), gen.subschema_for::<Duration>());
            props
        };
        res.object().required = btreeset!["change".to_owned(), "duration".to_owned()];
        res.into()
    }
}

//
// construction
//
impl<V: FloatBased + Vector<V::BaseFloat>> Rate<V> {
    /// Create a new Rate.
    /// Note that this function does not check if the duration is zero.
    #[inline]
    pub fn new(chg: V, dur: Duration) -> Self {
        if dur.is_zero() {
            return Self(_Rate::Diverged { chg });
        }
        Self(_Rate::Finite {
            chg: if dur == _unit_time() {
                chg
            } else {
                chg / &_dur_to_num_units::<V>(dur)
            },
            dur,
        })
    }

    /// Create a new Rate.
    /// If the duration is zero, this function returns `None`.
    #[inline]
    pub fn safe_new(chg: V, dur: Duration) -> Option<Self> {
        if dur.is_zero() {
            None
        } else {
            Some(Self::new(chg, dur))
        }
    }

    /// Create a new Rate with annual(365 days) duration
    ///
    /// Note that this assumes a year is 365 days.
    /// In financial context, this means that ACT/365 fixed convention
    /// is assumed when this rate is used to get the change.
    #[inline]
    pub fn with_annual(chg: V) -> Self {
        Self::new(chg, Duration::with_secs(60 * 60 * 24 * 365))
    }
}

impl<V: FloatBased + Vector<V::BaseFloat>> Zero for Rate<V> {
    #[inline]
    fn zero() -> Self {
        Self(_Rate::Finite {
            chg: Zero::zero(),
            dur: _unit_time(),
        })
    }

    #[inline]
    fn is_zero(&self) -> bool {
        match &self.0 {
            _Rate::Finite { chg, .. } => chg.is_zero(),
            _Rate::Diverged { .. } => false,
        }
    }
}

//
// comparison
//
impl<V: PartialOrd + FloatBased + Vector<V::BaseFloat>> PartialEq for Rate<V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use _Rate::*;
        match (&self.0, &other.0) {
            (Finite { chg: lhs, .. }, Finite { chg: rhs, .. }) => lhs == rhs,
            (Diverged { chg: lhs }, Diverged { chg: rhs }) => {
                if lhs.is_zero() || rhs.is_zero() {
                    // NaN caused by 0/0
                    false
                } else {
                    (lhs < &Zero::zero()) == (rhs < &Zero::zero())
                }
            }
            _ => false,
        }
    }
}

impl<V: PartialOrd + FloatBased + Vector<V::BaseFloat>> PartialOrd for Rate<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use _Rate::*;
        match (&self.0, &other.0) {
            (Finite { chg: lhs, .. }, Finite { chg: rhs, .. }) => lhs.partial_cmp(rhs),
            (Diverged { chg: lhs }, Diverged { chg: rhs }) => {
                if lhs.is_zero() || rhs.is_zero() {
                    // NaN caused by 0/0
                    None
                } else {
                    match (lhs < &Zero::zero(), rhs < &Zero::zero()) {
                        (true, true) => Some(std::cmp::Ordering::Equal),
                        (true, false) => Some(std::cmp::Ordering::Less),
                        (false, true) => Some(std::cmp::Ordering::Greater),
                        (false, false) => Some(std::cmp::Ordering::Equal),
                    }
                }
            }
            (Diverged { chg: lhs }, Finite { .. }) => {
                if lhs.is_zero() {
                    None
                } else {
                    lhs.partial_cmp(&Zero::zero())
                }
            }
            (Finite { .. }, Diverged { chg: rhs }) => {
                if rhs.is_zero() {
                    None
                } else {
                    V::zero().partial_cmp(rhs)
                }
            }
        }
    }
}

//
// methods
//
impl<V: FloatBased> FloatBased for Rate<V> {
    type BaseFloat = V::BaseFloat;
}

impl<V: FloatBased + Vector<V::BaseFloat>> Rate<V> {
    /// Get the change per given duration.
    ///
    /// # Examples
    /// ```
    /// use qrs_core::chrono::Duration;
    ///
    /// let vel = qrs_core::chrono::Rate::new(10.0, Duration::with_secs(1));
    ///
    /// assert_eq!(vel.to_change(Duration::with_mins(1)), 600.0);
    /// ```
    #[inline]
    pub fn to_change(self, dur: Duration) -> V {
        match self.0 {
            _Rate::Diverged { chg } => {
                let zero = &<V::BaseFloat as Zero>::zero();
                if dur < Duration::zero() {
                    -chg / zero
                } else {
                    chg / zero
                }
            }
            _Rate::Finite { chg, .. } => {
                let mult = _dur_to_num_units::<V>(dur);
                chg * &mult
            }
        }
    }

    /// Get the change per a year(365 days).
    #[inline]
    pub fn to_annual_change(self) -> V {
        self.to_change(Duration::with_secs(60 * 60 * 24 * 365))
    }

    /// Check that zero-division occurred.
    ///
    /// # Examples
    /// ```
    /// use qrs_core::chrono::Duration;
    /// use num::Zero;
    ///
    /// let vel = qrs_core::chrono::Rate::new(10.0, Duration::with_secs(1));
    /// assert!(!vel.is_diverged());
    ///
    /// let vel = qrs_core::chrono::Rate::new(10.0, Duration::zero());
    /// assert!(vel.is_diverged());
    /// ```
    #[inline]
    pub fn is_diverged(&self) -> bool {
        matches!(self.0, _Rate::Diverged { .. })
    }
}

//
// operators
//
// neg
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Neg for Rate<V> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        match self {
            Rate(_Rate::Diverged { chg }) => Rate(_Rate::Diverged { chg: -chg }),
            Rate(_Rate::Finite { chg, dur }) => Rate(_Rate::Finite { chg: -chg, dur }),
        }
    }
}

// add
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Add for Rate<V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self + &rhs
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Add<&Self> for Rate<V> {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: &Self) -> Self {
        self += rhs;
        self
    }
}

// add assign
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign<&Self> for Rate<V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        match (&mut self.0, &rhs.0) {
            (_Rate::Diverged { .. }, _) => {}
            (_, _Rate::Diverged { .. }) => *self = rhs.clone(),
            (_Rate::Finite { chg, .. }, _Rate::Finite { chg: rhs_chg, .. }) => *chg += &rhs_chg,
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign for Rate<V> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

// sub
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Sub for Rate<V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self - &rhs
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Sub<&Self> for Rate<V> {
    type Output = Self;

    #[inline]
    fn sub(mut self, rhs: &Self) -> Self {
        self -= rhs;
        self
    }
}

// sub assign
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::SubAssign<&Self> for Rate<V> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        match (&mut self.0, &rhs.0) {
            (_Rate::Diverged { .. }, _) => {}
            (_, _Rate::Diverged { .. }) => *self = -rhs.clone(),
            (_Rate::Finite { chg, .. }, _Rate::Finite { chg: rhs_chg, .. }) => *chg -= &rhs_chg,
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::SubAssign for Rate<V> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

// mul
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Mul<Duration> for Rate<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> V {
        self.to_change(rhs)
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Mul<&Duration> for Rate<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: &Duration) -> V {
        self.to_change(*rhs)
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::Mul<&K> for Rate<V> {
    type Output = Self;

    #[inline]
    fn mul(mut self, rhs: &K) -> Self {
        self *= rhs;
        self
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::MulAssign<&K> for Rate<V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &K) {
        match &mut self.0 {
            _Rate::Diverged { chg } => *chg *= rhs,
            _Rate::Finite { chg, .. } => *chg *= rhs,
        }
    }
}

// div
impl<K: Arithmetic, V: Vector<K>> std::ops::Div<&K> for Rate<V> {
    type Output = Self;

    #[inline]
    fn div(mut self, rhs: &K) -> Self {
        self /= rhs;
        self
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::DivAssign<&K> for Rate<V> {
    #[inline]
    fn div_assign(&mut self, rhs: &K) {
        match &mut self.0 {
            _Rate::Diverged { chg } => *chg /= rhs,
            _Rate::Finite { chg, .. } => *chg /= rhs,
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Div<Duration> for Rate<V> {
    type Output = Rate<Rate<V>>;

    #[inline]
    fn div(self, rhs: Duration) -> Self::Output {
        Rate::new(self, rhs)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use rstest::rstest;
    use rstest_reuse::{apply, template};
    use static_assertions::assert_impl_all;

    use super::*;

    #[template]
    #[rstest]
    fn cases_for_single(
        #[values(
            Rate::new(10.0, Duration::with_secs(1)),
            Rate::new(-10.0, Duration::with_secs(1)),
            Rate::new(10.0, Duration::with_secs(-2)),
            Rate::new(-10.0, Duration::with_secs(-2)),
            Rate::new(15.0, Duration::with_mins(2)),
            Rate::new(-15.0, Duration::with_mins(2)),
            Rate::new(10.0, Duration::zero()),
            Rate::new(-10.0, Duration::zero())
        )]
        vel: Rate<f64>,
    ) {
    }

    #[template]
    #[rstest]
    fn cases_for_symmetric_pair(
        #[values(
            // Rate::new(10.0, Duration::with_secs(1)),
            // Rate::new(-10.0, Duration::with_secs(1)),
            // Rate::new(10.0, Duration::with_secs(-2)),
            // Rate::new(-10.0, Duration::with_secs(-2)),
            // Rate::new(15.0, Duration::with_mins(2)),
            // Rate::new(-15.0, Duration::with_mins(2)),
            Rate::new(10.0, Duration::zero()),
            // Rate::new(-10.0, Duration::zero())
        )]
        lhs: Rate<f64>,
        #[values(
            // Rate::new(10.0, Duration::with_secs(1)),
            // Rate::new(-10.0, Duration::with_secs(1)),
            // Rate::new(10.0, Duration::with_secs(-2)),
            // Rate::new(-10.0, Duration::with_secs(-2)),
            // Rate::new(15.0, Duration::with_mins(2)),
            // Rate::new(-15.0, Duration::with_mins(2)),
            Rate::new(10.0, Duration::zero()),
            // Rate::new(-10.0, Duration::zero())
        )]
        rhs: Rate<f64>,
    ) {
    }

    #[test]
    fn test_is_vector() {
        assert_impl_all!(Rate<f32>: Vector<f32>);
        assert_impl_all!(Rate<f64>: Vector<f64>);
    }

    #[test]
    fn test_new() {
        let vel = Rate::new(10.0, Duration::with_secs(1));
        match vel.0 {
            _Rate::Finite { chg, dur } => {
                assert_abs_diff_eq!(chg, 10.0, epsilon = 1e-15);
                assert_eq!(dur, Duration::with_secs(1));
            }
            _ => panic!(),
        }
        assert!(!vel.is_diverged());
        assert_eq!(vel, Rate::safe_new(10.0, Duration::with_secs(1)).unwrap());

        let vel = Rate::new(10.0, Duration::with_secs(2));
        match vel.0 {
            _Rate::Finite { chg, dur } => {
                assert_abs_diff_eq!(chg, 5.0, epsilon = 1e-15);
                assert_eq!(dur, Duration::with_secs(2));
            }
            _ => panic!(),
        }
        assert!(!vel.is_diverged());
        assert_eq!(vel, Rate::safe_new(10.0, Duration::with_secs(2)).unwrap());

        let vel = Rate::new(10.0, Duration::with_secs(0));
        match vel.0 {
            _Rate::Diverged { chg } => {
                assert_abs_diff_eq!(chg, 10.0, epsilon = 1e-15);
            }
            _ => panic!(),
        }
        assert!(Rate::safe_new(10.0, Duration::with_secs(0)).is_none());
    }

    #[test]
    fn test_serialize() {
        let vel = Rate::new(10.0, Duration::with_secs(1));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT1S"}"#);

        let vel = Rate::new(60.0, Duration::with_mins(2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":60.0,"duration":"PT2M"}"#);

        let vel = Rate::new(60.0, Duration::with_mins(-2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":60.0,"duration":"-PT2M"}"#);

        let vel = Rate::new(10.0, Duration::with_secs(0));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT0S"}"#);

        let vel = Rate::new(-12.5, Duration::zero());
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":-12.5,"duration":"PT0S"}"#);
    }

    #[test]
    fn test_deserialize() {
        let vel: Rate<f64> = serde_json::from_str(r#"{"change":10.0,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Rate::new(10.0, Duration::with_secs(1)));

        let vel: Rate<f64> = serde_json::from_str(r#"{"change":0.5,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Rate::new(60.0, Duration::with_mins(2)));

        let vel: Rate<f64> = serde_json::from_str(r#"{"change":-0.5,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Rate::new(60.0, Duration::with_mins(-2)));

        let vel: Rate<f64> = serde_json::from_str(r#"{"change": 60.0,"duration":"PT2M"}"#).unwrap();
        assert_eq!(vel, Rate::new(60.0, Duration::with_mins(2)));

        let vel: Rate<f64> = serde_json::from_str(r#"{"change":60.0,"duration":"PT-2M"}"#).unwrap();
        assert_eq!(vel, Rate::new(60.0, Duration::with_mins(-2)));

        let vel: Rate<f64> = serde_json::from_str(r#"{"change":10.0,"duration":"PT0S"}"#).unwrap();
        match vel.0 {
            _Rate::Diverged { chg } => assert_eq!(chg, 10.0),
            _ => panic!(),
        }
        assert!(vel.is_diverged());

        let vel: Rate<f64> = serde_json::from_str(r#"{"change":-12.5,"duration":"PT0S"}"#).unwrap();
        match vel.0 {
            _Rate::Diverged { chg } => assert_eq!(chg, -12.5),
            _ => panic!(),
        }
        assert!(vel.is_diverged());
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_partial_cmp(lhs: Rate<f64>, rhs: Rate<f64>) {
        let act = lhs.partial_cmp(&rhs);
        let durs = vec![
            Duration::with_secs(1),
            Duration::with_secs(-2),
            Duration::with_mins(2),
            Duration::with_mins(-1),
        ];
        for dur in durs {
            let lhs_chg = lhs.to_change(dur);
            let rhs_chg = rhs.to_change(dur);
            assert_eq!(lhs == rhs, lhs_chg == rhs_chg);

            let exp = lhs_chg.partial_cmp(&rhs_chg);
            if dur < Duration::zero() {
                assert_eq!(act, exp.map(|o| o.reverse()));
            } else {
                assert_eq!(act, exp);
            }
        }
    }

    #[test]
    fn test_to_change() {
        let vel = Rate::new(10.0, Duration::with_secs(1));
        assert_eq!(vel.to_change(Duration::with_secs(1)), 10.0);
        assert_eq!(vel.to_change(Duration::with_secs(-1)), -10.0);
        assert_eq!(vel.to_change(Duration::with_mins(1)), 600.0);
        assert_eq!(vel.to_change(Duration::with_mins(-1)), -600.0);
        assert_eq!(vel.to_change(Duration::with_mins(2)), 1200.0);
        assert_eq!(vel.to_change(Duration::with_mins(-2)), -1200.0);
        assert_eq!(vel.to_change(Duration::zero()), 0.0);

        let vel = Rate::new(-3600.0, Duration::with_mins(2));
        assert_eq!(vel.to_change(Duration::with_secs(1)), -30.0);
        assert_eq!(vel.to_change(Duration::with_secs(-1)), 30.0);
        assert_eq!(vel.to_change(Duration::with_mins(1)), -1800.0);
        assert_eq!(vel.to_change(Duration::with_mins(-1)), 1800.0);
        assert_eq!(vel.to_change(Duration::with_mins(2)), -3600.0);
        assert_eq!(vel.to_change(Duration::with_mins(-2)), 3600.0);
        assert_eq!(vel.to_change(Duration::zero()), 0.0);
        assert_eq!(vel.to_change(Duration::zero()), 0.0);
    }

    #[apply(cases_for_single)]
    fn test_neg(vel: Rate<f64>) {
        let act = -vel;
        if vel.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        let durs = vec![
            Duration::with_secs(1),
            Duration::with_secs(-2),
            Duration::with_mins(2),
            Duration::with_mins(-1),
            Duration::zero(),
        ];
        for dur in durs {
            assert_abs_diff_eq!(act.to_change(dur), -(vel.to_change(dur)), epsilon = 1e-15);
        }
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_add(lhs: Rate<f64>, rhs: Rate<f64>) {
        let act = lhs + rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        let durs = vec![
            Duration::with_secs(1),
            Duration::with_secs(-2),
            Duration::with_mins(2),
            Duration::with_mins(-1),
            Duration::zero(),
        ];
        for dur in durs {
            assert_abs_diff_eq!(
                act.to_change(dur),
                lhs.to_change(dur) + rhs.to_change(dur),
                epsilon = 1e-15
            );
        }
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_add_assign(lhs: Rate<f64>, rhs: Rate<f64>) {
        let mut act = lhs;
        act += &rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        assert_eq!(act, lhs + rhs);
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_sub(lhs: Rate<f64>, rhs: Rate<f64>) {
        let act = lhs - rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        let durs = vec![
            Duration::with_secs(1),
            Duration::with_secs(-2),
            Duration::with_mins(2),
            Duration::with_mins(-1),
            Duration::zero(),
        ];
        for dur in durs {
            assert_abs_diff_eq!(
                act.to_change(dur),
                lhs.to_change(dur) - rhs.to_change(dur),
                epsilon = 1e-15
            );
        }
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_sub_assign(lhs: Rate<f64>, rhs: Rate<f64>) {
        let mut act = lhs;
        act -= &rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        assert_eq!(act, lhs - rhs);
    }

    #[apply(cases_for_single)]
    fn test_mul_with_dur(vel: Rate<f64>) {
        let durs = vec![
            Duration::with_secs(1),
            Duration::with_secs(-2),
            Duration::with_mins(2),
            Duration::with_mins(-1),
            Duration::zero(),
        ];
        for dur in durs {
            let act = vel * dur;
            let exp = vel.to_change(dur);

            assert_eq!(act.total_cmp(&exp), std::cmp::Ordering::Equal);
        }
    }

    #[apply(cases_for_single)]
    fn test_mul_with_scalar(vel: Rate<f64>) {
        let scals = vec![0.0, 1.0, -1.0, 2.0, -2.0];

        for scal in scals {
            let act = vel * &scal;

            if vel.is_diverged() {
                assert!(act.is_diverged());
            } else {
                let durs = vec![
                    Duration::with_secs(1),
                    Duration::with_secs(-2),
                    Duration::with_mins(2),
                    Duration::with_mins(-1),
                    Duration::zero(),
                ];
                for dur in durs {
                    assert_abs_diff_eq!(
                        act.to_change(dur),
                        scal * vel.to_change(dur),
                        epsilon = 1e-15
                    );
                }
            }
        }
    }

    #[apply(cases_for_single)]
    fn test_mul_assign_with_scalar(vel: Rate<f64>) {
        let scals = vec![0.0, 1.0, -1.0, 2.0, -2.0];

        for scal in scals {
            let mut act = vel;
            act *= &scal;

            let durs = vec![
                Duration::with_secs(1),
                Duration::with_secs(-2),
                Duration::with_mins(2),
                Duration::with_mins(-1),
                Duration::zero(),
            ];
            for dur in durs {
                assert_eq!(
                    act.to_change(dur).total_cmp(&(scal * vel.to_change(dur))),
                    std::cmp::Ordering::Equal
                );
            }
        }
    }

    #[apply(cases_for_single)]
    fn test_div_with_scalar(vel: Rate<f64>) {
        let scals = vec![1.0, -1.0, 2.0, -2.0];

        for scal in scals {
            let act = vel / &scal;

            if vel.is_diverged() {
                assert!(act.is_diverged());
            } else {
                let durs = vec![
                    Duration::with_secs(1),
                    Duration::with_secs(-2),
                    Duration::with_mins(2),
                    Duration::with_mins(-1),
                    Duration::zero(),
                ];
                for dur in durs {
                    assert_abs_diff_eq!(
                        act.to_change(dur),
                        vel.to_change(dur) / scal,
                        epsilon = 1e-15
                    );
                }
            }
        }
    }

    #[apply(cases_for_single)]
    fn test_div_assign_with_scalar(vel: Rate<f64>) {
        let scals = vec![1.0, -1.0, 2.0, -2.0];

        for scal in scals {
            let mut act = vel;
            act /= &scal;

            let durs = vec![
                Duration::with_secs(1),
                Duration::with_secs(-2),
                Duration::with_mins(2),
                Duration::with_mins(-1),
                Duration::zero(),
            ];
            for dur in durs {
                assert_eq!(
                    act.to_change(dur).total_cmp(&(vel.to_change(dur) / scal)),
                    std::cmp::Ordering::Equal
                );
            }
        }
    }
}
