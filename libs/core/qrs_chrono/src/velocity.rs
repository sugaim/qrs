use qrs_math::num::{Arithmetic, FloatBased, Vector, Zero};
#[cfg(feature = "serde")]
use schemars::{schema::Schema, JsonSchema};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::Duration;

// -----------------------------------------------------------------------------
// _Velocity
//
#[derive(Debug, Clone, Copy)]
enum _Velocity<V> {
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
// Velocity
//

/// Change per given duration.
///
/// The main purpose of this type is to control arithmetics
/// between numbers and durations,
/// which are necessary under the low layer of implementations,
/// such as interpolation whose x-axis is datetime.
///
/// # Examples
/// ```
/// use qrs_chrono::Duration;
///
/// let vel = 10.0 / Duration::with_secs(1);
/// let chg = vel * Duration::with_mins(1);
///
/// assert_eq!(chg, 600.0);
/// ```
///
#[derive(Debug, Clone, Copy)]
pub struct Velocity<V>(_Velocity<V>);

//
// display, serde
//
#[cfg(feature = "serde")]
impl<V: FloatBased + Vector<V::BaseFloat> + Serialize> Serialize for Velocity<V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let orig_chg = match &self.0 {
            _Velocity::Diverged { chg } => chg.clone(),
            _Velocity::Finite { chg, dur } => chg.clone() * &_dur_to_num_units::<V>(*dur),
        };
        #[derive(Serialize)]
        struct VelocityHelper<V> {
            change: V,
            duration: Duration,
        }

        VelocityHelper {
            change: orig_chg,
            duration: match self.0 {
                _Velocity::Diverged { .. } => Duration::zero(),
                _Velocity::Finite { dur, .. } => dur,
            },
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, V: FloatBased + Vector<V::BaseFloat> + Deserialize<'de>> Deserialize<'de>
    for Velocity<V>
{
    fn deserialize<D>(deserializer: D) -> Result<Velocity<V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VelocityHelper<V> {
            change: V,
            duration: Duration,
        }
        let helper = VelocityHelper::deserialize(deserializer)?;
        Ok(Velocity::new(helper.change, helper.duration))
    }
}

#[cfg(feature = "serde")]
impl<V: JsonSchema> JsonSchema for Velocity<V> {
    fn schema_name() -> String {
        format!("Velocity_for_{}", V::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_chrono::Velocity<{}>", V::schema_id()).into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use maplit::btreeset;

        let mut res = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            ..Default::default()
        };
        res.metadata().description = Some("A change per given duration.".to_string());
        res.object().properties = {
            let mut props = std::collections::BTreeMap::default();

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
impl<V: FloatBased + Vector<V::BaseFloat>> Velocity<V> {
    /// Create a new Rate.
    /// Note that this function does not check if the duration is zero.
    #[inline]
    pub fn new(chg: V, dur: Duration) -> Self {
        if dur.is_zero() {
            return Self(_Velocity::Diverged { chg });
        }
        Self(_Velocity::Finite {
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
}

impl<V: FloatBased + Vector<V::BaseFloat>> Zero for Velocity<V> {
    #[inline]
    fn zero() -> Self {
        Self(_Velocity::Finite {
            chg: Zero::zero(),
            dur: _unit_time(),
        })
    }

    #[inline]
    fn is_zero(&self) -> bool {
        match &self.0 {
            _Velocity::Finite { chg, .. } => chg.is_zero(),
            _Velocity::Diverged { .. } => false,
        }
    }
}

//
// comparison
//
impl<V: PartialOrd + FloatBased + Vector<V::BaseFloat>> PartialEq for Velocity<V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use _Velocity::*;
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

impl<V: PartialOrd + FloatBased + Vector<V::BaseFloat>> PartialOrd for Velocity<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use _Velocity::*;
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
impl<V: FloatBased> FloatBased for Velocity<V> {
    type BaseFloat = V::BaseFloat;
}

impl<V: FloatBased + Vector<V::BaseFloat>> Velocity<V> {
    /// Get the change per given duration.
    ///
    /// # Examples
    /// ```
    /// use qrs_chrono::Duration;
    ///
    /// let vel = qrs_chrono::Velocity::new(10.0, Duration::with_secs(1));
    ///
    /// assert_eq!(vel.to_change(Duration::with_mins(1)), 600.0);
    /// ```
    #[inline]
    pub fn to_change(self, dur: Duration) -> V {
        match self.0 {
            _Velocity::Diverged { chg } => {
                let zero = &<V::BaseFloat as Zero>::zero();
                if dur < Duration::zero() {
                    -chg / zero
                } else {
                    chg / zero
                }
            }
            _Velocity::Finite { chg, .. } => {
                let mult = _dur_to_num_units::<V>(dur);
                chg * &mult
            }
        }
    }

    /// Check that zero-division occurred.
    ///
    /// # Examples
    /// ```
    /// use qrs_chrono::Duration;
    /// use qrs_math::num::Zero;
    ///
    /// let vel = qrs_chrono::Velocity::new(10.0, Duration::with_secs(1));
    /// assert!(!vel.is_diverged());
    ///
    /// let vel = qrs_chrono::Velocity::new(10.0, Duration::zero());
    /// assert!(vel.is_diverged());
    /// ```
    #[inline]
    pub fn is_diverged(&self) -> bool {
        matches!(self.0, _Velocity::Diverged { .. })
    }
}

//
// operators
//
// neg
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Neg for Velocity<V> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        match self {
            Velocity(_Velocity::Diverged { chg }) => Velocity(_Velocity::Diverged { chg: -chg }),
            Velocity(_Velocity::Finite { chg, dur }) => {
                Velocity(_Velocity::Finite { chg: -chg, dur })
            }
        }
    }
}

// add
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Add for Velocity<V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self + &rhs
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Add<&Self> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: &Self) -> Self {
        self += rhs;
        self
    }
}

// add assign
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign<&Self> for Velocity<V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        match (&mut self.0, &rhs.0) {
            (_Velocity::Diverged { .. }, _) => {}
            (_, _Velocity::Diverged { .. }) => *self = rhs.clone(),
            (_Velocity::Finite { chg, .. }, _Velocity::Finite { chg: rhs_chg, .. }) => {
                *chg += &rhs_chg
            }
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign for Velocity<V> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

// sub
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Sub for Velocity<V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self - &rhs
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Sub<&Self> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn sub(mut self, rhs: &Self) -> Self {
        self -= rhs;
        self
    }
}

// sub assign
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::SubAssign<&Self> for Velocity<V> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        match (&mut self.0, &rhs.0) {
            (_Velocity::Diverged { .. }, _) => {}
            (_, _Velocity::Diverged { .. }) => *self = -rhs.clone(),
            (_Velocity::Finite { chg, .. }, _Velocity::Finite { chg: rhs_chg, .. }) => {
                *chg -= &rhs_chg
            }
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::SubAssign for Velocity<V> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

// mul
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Mul<Duration> for Velocity<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> V {
        self.to_change(rhs)
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Mul<&Duration> for Velocity<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: &Duration) -> V {
        self.to_change(*rhs)
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::Mul<&K> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn mul(mut self, rhs: &K) -> Self {
        self *= rhs;
        self
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::MulAssign<&K> for Velocity<V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &K) {
        match &mut self.0 {
            _Velocity::Diverged { chg } => *chg *= rhs,
            _Velocity::Finite { chg, .. } => *chg *= rhs,
        }
    }
}

// div
impl<K: Arithmetic, V: Vector<K>> std::ops::Div<&K> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn div(mut self, rhs: &K) -> Self {
        self /= rhs;
        self
    }
}
impl<K: Arithmetic, V: Vector<K>> std::ops::DivAssign<&K> for Velocity<V> {
    #[inline]
    fn div_assign(&mut self, rhs: &K) {
        match &mut self.0 {
            _Velocity::Diverged { chg } => *chg /= rhs,
            _Velocity::Finite { chg, .. } => *chg /= rhs,
        }
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Div<Duration> for Velocity<V> {
    type Output = Velocity<Velocity<V>>;

    #[inline]
    fn div(self, rhs: Duration) -> Self::Output {
        Velocity::new(self, rhs)
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
            Velocity::new(10.0, Duration::with_secs(1)),
            Velocity::new(-10.0, Duration::with_secs(1)),
            Velocity::new(10.0, Duration::with_secs(-2)),
            Velocity::new(-10.0, Duration::with_secs(-2)),
            Velocity::new(15.0, Duration::with_mins(2)),
            Velocity::new(-15.0, Duration::with_mins(2)),
            Velocity::new(10.0, Duration::zero()),
            Velocity::new(-10.0, Duration::zero())
        )]
        vel: Rate<f64>,
    ) {
    }

    #[template]
    #[rstest]
    fn cases_for_symmetric_pair(
        #[values(
            Velocity::new(10.0, Duration::with_secs(1)),
            Velocity::new(-10.0, Duration::with_secs(1)),
            Velocity::new(10.0, Duration::with_secs(-2)),
            Velocity::new(-10.0, Duration::with_secs(-2)),
            Velocity::new(15.0, Duration::with_mins(2)),
            Velocity::new(-15.0, Duration::with_mins(2)),
            Velocity::new(10.0, Duration::zero()),
            Velocity::new(-10.0, Duration::zero())
        )]
        lhs: Rate<f64>,
        #[values(
            Velocity::new(10.0, Duration::with_secs(1)),
            Velocity::new(-10.0, Duration::with_secs(1)),
            Velocity::new(10.0, Duration::with_secs(-2)),
            Velocity::new(-10.0, Duration::with_secs(-2)),
            Velocity::new(15.0, Duration::with_mins(2)),
            Velocity::new(-15.0, Duration::with_mins(2)),
            Velocity::new(10.0, Duration::zero()),
            Velocity::new(-10.0, Duration::zero())
        )]
        rhs: Velocity<f64>,
    ) {
    }

    #[test]
    fn test_is_vector() {
        assert_impl_all!(Velocity<f32>: Vector<f32>);
        assert_impl_all!(Velocity<f64>: Vector<f64>);
    }

    #[test]
    fn test_new() {
        let vel = Velocity::new(10.0, Duration::with_secs(1));
        match vel.0 {
            _Velocity::Finite { chg, dur } => {
                assert_abs_diff_eq!(chg, 10.0, epsilon = 1e-15);
                assert_eq!(dur, Duration::with_secs(1));
            }
            _ => panic!(),
        }
        assert!(!vel.is_diverged());
        assert_eq!(
            vel,
            Velocity::safe_new(10.0, Duration::with_secs(1)).unwrap()
        );

        let vel = Velocity::new(10.0, Duration::with_secs(2));
        match vel.0 {
            _Velocity::Finite { chg, dur } => {
                assert_abs_diff_eq!(chg, 5.0, epsilon = 1e-15);
                assert_eq!(dur, Duration::with_secs(2));
            }
            _ => panic!(),
        }
        assert!(!vel.is_diverged());
        assert_eq!(
            vel,
            Velocity::safe_new(10.0, Duration::with_secs(2)).unwrap()
        );

        let vel = Velocity::new(10.0, Duration::with_secs(0));
        match vel.0 {
            _Velocity::Diverged { chg } => {
                assert_abs_diff_eq!(chg, 10.0, epsilon = 1e-15);
            }
            _ => panic!(),
        }
        assert!(Velocity::safe_new(10.0, Duration::with_secs(0)).is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize() {
        let vel = Velocity::new(10.0, Duration::with_secs(1));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT1S"}"#);

        let vel = Velocity::new(60.0, Duration::with_mins(2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":60.0,"duration":"PT2M"}"#);

        let vel = Velocity::new(60.0, Duration::with_mins(-2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":60.0,"duration":"-PT2M"}"#);

        let vel = Velocity::new(10.0, Duration::with_secs(0));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT0S"}"#);

        let vel = Velocity::new(-12.5, Duration::zero());
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":-12.5,"duration":"PT0S"}"#);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deserialize() {
        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":10.0,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Velocity::new(10.0, Duration::with_secs(1)));

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":0.5,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Velocity::new(60.0, Duration::with_mins(2)));

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":-0.5,"duration":"PT1S"}"#).unwrap();
        assert_eq!(vel, Velocity::new(60.0, Duration::with_mins(-2)));

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change": 60.0,"duration":"PT2M"}"#).unwrap();
        assert_eq!(vel, Velocity::new(60.0, Duration::with_mins(2)));

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":60.0,"duration":"PT-2M"}"#).unwrap();
        assert_eq!(vel, Velocity::new(60.0, Duration::with_mins(-2)));

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":10.0,"duration":"PT0S"}"#).unwrap();
        match vel.0 {
            _Velocity::Diverged { chg } => assert_eq!(chg, 10.0),
            _ => panic!(),
        }
        assert!(vel.is_diverged());

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":-12.5,"duration":"PT0S"}"#).unwrap();
        match vel.0 {
            _Velocity::Diverged { chg } => assert_eq!(chg, -12.5),
            _ => panic!(),
        }
        assert!(vel.is_diverged());
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_partial_cmp(lhs: Velocity<f64>, rhs: Velocity<f64>) {
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
        let vel = Velocity::new(10.0, Duration::with_secs(1));
        assert_eq!(vel.to_change(Duration::with_secs(1)), 10.0);
        assert_eq!(vel.to_change(Duration::with_secs(-1)), -10.0);
        assert_eq!(vel.to_change(Duration::with_mins(1)), 600.0);
        assert_eq!(vel.to_change(Duration::with_mins(-1)), -600.0);
        assert_eq!(vel.to_change(Duration::with_mins(2)), 1200.0);
        assert_eq!(vel.to_change(Duration::with_mins(-2)), -1200.0);
        assert_eq!(vel.to_change(Duration::zero()), 0.0);

        let vel = Velocity::new(-3600.0, Duration::with_mins(2));
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
    fn test_neg(vel: Velocity<f64>) {
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
    fn test_add(lhs: Velocity<f64>, rhs: Velocity<f64>) {
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
    fn test_add_assign(lhs: Velocity<f64>, rhs: Velocity<f64>) {
        let mut act = lhs;
        act += &rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        assert_eq!(act, lhs + rhs);
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_sub(lhs: Velocity<f64>, rhs: Velocity<f64>) {
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
    fn test_sub_assign(lhs: Velocity<f64>, rhs: Velocity<f64>) {
        let mut act = lhs;
        act -= &rhs;
        if lhs.is_diverged() || rhs.is_diverged() {
            assert!(act.is_diverged());
            return;
        }
        assert_eq!(act, lhs - rhs);
    }

    #[apply(cases_for_single)]
    fn test_mul_with_dur(vel: Velocity<f64>) {
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
    fn test_mul_with_scalar(vel: Velocity<f64>) {
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
    fn test_mul_assign_with_scalar(vel: Velocity<f64>) {
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
    fn test_div_with_scalar(vel: Velocity<f64>) {
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
    fn test_div_assign_with_scalar(vel: Velocity<f64>) {
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
