use num::{One, Zero};
use serde::{Deserialize, Serialize};

use crate::math::num::{Arithmetic, FloatBased, Scalar, Vector};

use super::Duration;

// -----------------------------------------------------------------------------
// Velocity
//

/// A velocity, which is a change per given duration.
#[derive(Debug, Clone, Copy, Hash)]
pub struct Velocity<V> {
    chg: V,
    is_diverged: bool,
}

#[inline]
fn _unit_time() -> Duration {
    Duration::with_secs(1)
}

#[inline]
fn _dur2cnt<T: num::Float + Arithmetic>(dur: Duration) -> T {
    T::from(dur.secs()).unwrap()
        + T::from(dur.subsec_nanos()).unwrap() / T::from(1_000_000_000).unwrap()
}

//
// display, serde
//

impl<'de, V: FloatBased + Vector<V::BaseFloat> + Serialize> Serialize for Velocity<V>
where
    V::BaseFloat: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct VelocityHelper<'a, V> {
            change: &'a V,
            duration: Duration,
        }

        VelocityHelper {
            change: &self.chg,
            duration: if self.is_diverged {
                Duration::zero()
            } else {
                _unit_time()
            },
        }
        .serialize(serializer)
    }
}

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

//
// construction
//
impl<V: FloatBased + Vector<V::BaseFloat>> Velocity<V> {
    /// Create a new velocity.
    /// Note that this function does not check if the duration is zero.
    #[inline]
    pub fn new(chg: V, dur: Duration) -> Self {
        if dur.is_zero() {
            return Self {
                chg,
                is_diverged: true,
            };
        }
        Self {
            chg: if dur == _unit_time() {
                chg
            } else {
                chg / &_dur2cnt(dur)
            },
            is_diverged: false,
        }
    }

    /// Create a new velocity.
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
        Self {
            chg: V::zero(),
            is_diverged: false,
        }
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.chg.is_zero()
    }
}

//
// comparison
//
impl<V: PartialEq + FloatBased + Vector<V::BaseFloat>> PartialEq for Velocity<V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.is_diverged() || other.is_diverged() {
            return false;
        }
        self.chg == other.chg
    }
}

impl<V: PartialOrd + FloatBased + Vector<V::BaseFloat>> PartialOrd for Velocity<V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.is_diverged() || other.is_diverged() {
            return None;
        }
        self.chg.partial_cmp(&other.chg)
    }
}

//
// methods
//
impl<V: FloatBased + Vector<V::BaseFloat>> Velocity<V> {
    /// Get the change per given duration.
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::Duration;
    ///
    /// let vel = qcore::chrono::Velocity::new(10.0, Duration::with_secs(1));
    ///
    /// assert_eq!(vel.to_change(Duration::with_mins(1)), 600.0);
    /// ```
    #[inline]
    pub fn to_change(self, dur: Duration) -> V {
        let mult = _dur2cnt::<V::BaseFloat>(dur)
            / if self.is_diverged() {
                <V::BaseFloat as Zero>::zero()
            } else {
                <V::BaseFloat as One>::one()
            };
        if mult.is_one() {
            self.chg
        } else {
            self.chg * &mult
        }
    }

    /// Check that zero-division occurred.
    ///
    /// # Examples
    /// ```
    /// use qcore::chrono::Duration;
    /// use num::Zero;
    ///
    /// let vel = qcore::chrono::Velocity::new(10.0, Duration::with_secs(1));
    /// assert!(!vel.is_diverged());
    ///
    /// let vel = qcore::chrono::Velocity::new(10.0, Duration::zero());
    /// assert!(vel.is_diverged());
    /// ```
    #[inline]
    pub fn is_diverged(&self) -> bool {
        self.is_diverged
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
        Self {
            chg: -self.chg,
            is_diverged: self.is_diverged,
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
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Add<&Velocity<V>> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: &Self) -> Self {
        self += rhs;
        self
    }
}

// add assign
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign<&Velocity<V>> for Velocity<V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        if self.is_diverged() {
            return;
        }
        if rhs.is_diverged() {
            *self = rhs.clone();
            return;
        }
        self.chg += &rhs.chg;
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::AddAssign<Velocity<V>> for Velocity<V> {
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
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::Sub<&Velocity<V>> for Velocity<V> {
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
        if self.is_diverged() {
            return;
        }
        if rhs.is_diverged() {
            *self = -rhs.clone();
            return;
        }
        self.chg -= &rhs.chg;
    }
}
impl<V: FloatBased + Vector<V::BaseFloat>> std::ops::SubAssign<Velocity<V>> for Velocity<V> {
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
impl<K: Scalar, V: Vector<K>> std::ops::Mul<&K> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn mul(mut self, rhs: &K) -> Self {
        self *= rhs;
        self
    }
}
impl<K: Scalar, V: Vector<K>> std::ops::MulAssign<&K> for Velocity<V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &K) {
        self.chg *= rhs;
    }
}

// div
impl<K: Scalar, V: Vector<K>> std::ops::Div<&K> for Velocity<V> {
    type Output = Self;

    #[inline]
    fn div(mut self, rhs: &K) -> Self {
        self /= rhs;
        self
    }
}
impl<K: Scalar, V: Vector<K>> std::ops::DivAssign<&K> for Velocity<V> {
    #[inline]
    fn div_assign(&mut self, rhs: &K) {
        self.chg /= rhs;
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
        vel: Velocity<f64>,
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
        lhs: Velocity<f64>,
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
        assert_eq!(vel.chg, 10.0);
        assert!(!vel.is_diverged());
        assert_eq!(
            vel,
            Velocity::safe_new(10.0, Duration::with_secs(1)).unwrap()
        );

        let vel = Velocity::new(10.0, Duration::with_secs(2));
        assert_abs_diff_eq!(vel.chg, 5.0, epsilon = 1e-15);
        assert!(!vel.is_diverged());
        assert_eq!(
            vel,
            Velocity::safe_new(10.0, Duration::with_secs(2)).unwrap()
        );

        let vel = Velocity::new(10.0, Duration::with_secs(0));
        assert_eq!(vel.chg, 10.0);
        assert!(vel.is_diverged());
        assert!(Velocity::safe_new(10.0, Duration::with_secs(0)).is_none());
    }

    #[test]
    fn test_serialize() {
        let vel = Velocity::new(10.0, Duration::with_secs(1));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT1S"}"#);

        let vel = Velocity::new(60.0, Duration::with_mins(2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":0.5,"duration":"PT1S"}"#);

        let vel = Velocity::new(60.0, Duration::with_mins(-2));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":-0.5,"duration":"PT1S"}"#);

        let vel = Velocity::new(10.0, Duration::with_secs(0));
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":10.0,"duration":"PT0S"}"#);

        let vel = Velocity::new(-12.5, Duration::zero());
        let ser = serde_json::to_string(&vel).unwrap();
        assert_eq!(ser, r#"{"change":-12.5,"duration":"PT0S"}"#);
    }

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
        assert_eq!(vel.chg, 10.0);
        assert!(vel.is_diverged());

        let vel: Velocity<f64> =
            serde_json::from_str(r#"{"change":-12.5,"duration":"PT0S"}"#).unwrap();
        assert_eq!(vel.chg, -12.5);
        assert!(vel.is_diverged());
    }

    #[apply(cases_for_symmetric_pair)]
    fn test_partial_cmp(lhs: Velocity<f64>, rhs: Velocity<f64>) {
        let act = lhs.partial_cmp(&rhs);
        if lhs.is_diverged() || rhs.is_diverged() {
            assert_eq!(act, None);
            assert!(lhs != rhs);
            return;
        }
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
