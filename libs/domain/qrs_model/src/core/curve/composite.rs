use std::{
    borrow::Borrow,
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};

use qrs_chrono::DateTime;
use qrs_datasrc::{CacheableSrc, DataSrc, DebugTree, TakeSnapshot};
use qrs_finance::daycount::Act365fRate;
use qrs_math::num::{FloatBased, Zero};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// WeightedCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct WeightedCurve<C> {
    pub weight: f64,
    pub curve: C,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for WeightedCurve<C> {
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        Ok(self.curve.forward_rate(from, to)?
            * &<C::Value as FloatBased>::nearest_base_float_of(self.weight))
    }
}

// -----------------------------------------------------------------------------
// CompositeCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct CompositeCurve<C> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub components: HashMap<String, WeightedCurve<C>>,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for CompositeCurve<C> {
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        let mut sum = Zero::zero();
        for c in self.components.values() {
            let r = c.forward_rate(from, to)?;
            sum += &r;
        }
        Ok(sum)
    }
}

// -----------------------------------------------------------------------------
// CompositeCrvSrc
//
#[derive(Debug, Clone, DebugTree)]
pub struct CompositeCurveSrc<C, W> {
    #[debug_tree(subtree)]
    crv: C,
    #[debug_tree(subtree)]
    weight: W,
}

//
// construction
//
impl<C, W> CompositeCurveSrc<C, W> {
    pub fn new(crv: C, weight: W) -> Self {
        Self { crv, weight }
    }
}

//
// methods
//
impl<C, W> CompositeCurveSrc<C, W> {
    #[inline]
    pub fn weight_src(&self) -> &W {
        &self.weight
    }

    #[inline]
    pub fn curve_src(&self) -> &C {
        &self.crv
    }
}

impl<C, W, Sym> DataSrc<Sym> for CompositeCurveSrc<C, W>
where
    Sym: ?Sized,
    C: DataSrc<str>,
    W: DataSrc<Sym>,
    W::Output: Borrow<HashMap<String, f64>>,
    C::Output: YieldCurve,
{
    type Output = CompositeCurve<C::Output>;

    fn get(&self, req: &Sym) -> anyhow::Result<Self::Output> {
        let weights = self.weight.get(req)?;
        let components = weights
            .borrow()
            .iter()
            .map(|(name, weight)| {
                let c = self.crv.get(name)?;
                let weight = *weight;
                Ok((name.clone(), WeightedCurve { weight, curve: c }))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(CompositeCurve { components })
    }
}

impl<C, W, Sym> CacheableSrc<Sym> for CompositeCurveSrc<C, W>
where
    Sym: ?Sized,
    C: CacheableSrc<str>,
    W: CacheableSrc<Sym>,
    W::Output: Borrow<HashMap<String, f64>>,
    C::Output: YieldCurve,
{
    fn etag(&self, req: &Sym) -> anyhow::Result<String> {
        let mut hasher = DefaultHasher::new();
        let weights = self.weight.get_with_etag(req)?;
        weights.etag.hash(&mut hasher);
        let mut hash_vals: u64 = 0;
        for (name, _) in weights.data.borrow().iter() {
            let mut h = DefaultHasher::new();
            self.crv.etag(name)?.hash(&mut h);
            hash_vals = hash_vals.overflowing_add(h.finish()).0;
        }
        hash_vals.hash(&mut hasher);
        Ok(hasher.finish().to_string())
    }
}

impl<C, W, Sym> TakeSnapshot<Sym> for CompositeCurveSrc<C, W>
where
    Sym: ?Sized,
    C: TakeSnapshot<str>,
    W: TakeSnapshot<Sym>,
    W::Output: Borrow<HashMap<String, f64>>,
    C::Output: YieldCurve,
{
    type Snapshot = CompositeCurveSrc<C::Snapshot, W::Snapshot>;

    fn take_snapshot<'a, Rqs>(&self, rqs: Rqs) -> anyhow::Result<Self::Snapshot>
    where
        Sym: 'a,
        Rqs: IntoIterator<Item = &'a Sym>,
    {
        let rqs = rqs.into_iter().collect::<Vec<_>>();
        let mut crvs = HashSet::new();
        for rq in &rqs {
            crvs.extend(self.weight.get(rq)?.borrow().keys().cloned());
        }
        Ok(CompositeCurveSrc {
            crv: self.crv.take_snapshot(crvs.iter().map(|s| s.as_str()))?,
            weight: self.weight.take_snapshot(rqs)?,
        })
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use maplit::hashmap;
    use qrs_chrono::DateToDateTime;
    use qrs_datasrc::Response;

    use crate::core::curve::FlatCurve;

    use super::*;

    mockall::mock! {
        FlatCurve {}

        impl YieldCurve for FlatCurve {
            type Value = f64;

            fn forward_rate(
                &self,
                from: &DateTime,
                to: &DateTime,
            ) -> anyhow::Result<Act365fRate<f64>>;
        }
    }

    mockall::mock! {
        CSrc {}

        impl DebugTree for CSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> qrs_datasrc::TreeInfo;
        }

        impl DataSrc<str> for CSrc {
            type Output = FlatCurve<f64>;

            fn get(&self, req: &str) -> anyhow::Result<FlatCurve<f64>>;
        }

        impl CacheableSrc<str> for CSrc {
            fn etag(&self, req: &str) -> anyhow::Result<String>;
            fn get_with_etag(&self, req: &str) -> anyhow::Result<Response<FlatCurve<f64>>>;
        }
    }

    mockall::mock! {
        WSrc {}

        impl DebugTree for WSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> qrs_datasrc::TreeInfo;
        }

        impl DataSrc<str> for WSrc {
            type Output = HashMap<String, f64>;

            fn get(&self, req: &str) -> anyhow::Result<HashMap<String, f64>>;
        }

        impl CacheableSrc<str> for WSrc {
            fn etag(&self, req: &str) -> anyhow::Result<String>;
            fn get_with_etag(&self, req: &str) -> anyhow::Result<Response<HashMap<String, f64>>>;
        }
    }

    #[derive(Default)]
    struct CallCount {
        get: Option<usize>,
        get_with_etag: Option<usize>,
        etag: Option<usize>,
    }

    impl CallCount {
        fn zero() -> Self {
            Self {
                get: Some(0),
                get_with_etag: Some(0),
                etag: Some(0),
            }
        }
    }

    fn get_crv(nm: &str) -> anyhow::Result<FlatCurve<f64>> {
        match nm {
            "SOFR" => Ok(FlatCurve {
                rate: Act365fRate::from_rate(0.01),
            }),
            "FF_SPREAD" => Ok(FlatCurve {
                rate: Act365fRate::from_rate(0.005),
            }),
            _ => Err(anyhow!("Unknown curve: {}", nm)),
        }
    }
    fn get_weight(nm: &str) -> anyhow::Result<HashMap<String, f64>> {
        match nm {
            "USD" => Ok(hashmap! {
                "SOFR".to_string() => -1.0,
                "FF_SPREAD".to_string() => 2.0,
            }),
            _ => Err(anyhow!("Unknown currency: {}", nm)),
        }
    }

    impl MockCSrc {
        fn setup(&mut self, cnt: &CallCount) {
            //
            let get = self.expect_get().returning(get_crv);
            if let Some(n) = cnt.get {
                get.times(n);
            }

            //
            let etag = self
                .expect_etag()
                .returning(|nm| Ok(format!("etag-{}", nm)));
            if let Some(n) = cnt.etag {
                etag.times(n);
            }

            //
            let get_with_etag = self.expect_get_with_etag().returning(|nm| {
                let data = get_crv(nm)?;
                Ok(Response {
                    data,
                    etag: format!("etag-{}", nm),
                })
            });
            if let Some(n) = cnt.get_with_etag {
                get_with_etag.times(n);
            }
        }

        fn with_call_count(cnt: &CallCount) -> Self {
            let mut m = MockCSrc::new();
            m.setup(cnt);
            m
        }
    }

    impl MockWSrc {
        fn setup(&mut self, cnt: &CallCount) {
            //
            let get = self.expect_get().returning(get_weight);
            if let Some(n) = cnt.get {
                get.times(n);
            }

            //
            let get_with_etag = self.expect_get_with_etag().returning(|nm| {
                Ok(Response {
                    data: get_weight(nm)?,
                    etag: format!("etag-{}", nm),
                })
            });
            if let Some(n) = cnt.get_with_etag {
                get_with_etag.times(n);
            }

            //
            let etag = self.expect_etag().returning(|s| Ok(format!("etag-{}", s)));
            if let Some(n) = cnt.etag {
                etag.times(n);
            }
        }
        fn with_call_count(cnt: &CallCount) -> Self {
            let mut m = MockWSrc::new();
            m.setup(cnt);
            m
        }
    }

    fn d2dt() -> DateToDateTime {
        DateTime::builder()
            .with_hms(15, 30, 0)
            .with_parsed_timezone("+09:00")
    }

    #[test]
    fn test_weighted_curve() {
        let mut c = MockFlatCurve::new();
        c.expect_forward_rate()
            .once()
            .returning(|_, _| Ok(Act365fRate::from_rate(0.01)));
        let mut wc = WeightedCurve {
            weight: -2.0,
            curve: c,
        };
        let from = d2dt().with_ymd(2021, 1, 1).build().unwrap();
        let to = d2dt().with_ymd(2022, 1, 10).build().unwrap();

        let r = wc.forward_rate(&from, &to).unwrap();

        assert_eq!(r, Act365fRate::from_rate(-0.02));
        wc.curve.checkpoint();
    }

    #[test]
    fn test_composite_curve() {
        let mut c1 = MockFlatCurve::new();
        c1.expect_forward_rate()
            .once()
            .returning(|_, _| Ok(Act365fRate::from_rate(0.01)));
        let mut c2 = MockFlatCurve::new();
        c2.expect_forward_rate()
            .once()
            .returning(|_, _| Ok(Act365fRate::from_rate(0.02)));
        let mut cc = CompositeCurve {
            components: hashmap! {
                "SOFR".to_string() => WeightedCurve { weight: -1.0, curve: c1 },
                "FF_SPREAD".to_string() => WeightedCurve { weight: 2.0, curve: c2 },
            },
        };
        let from = d2dt().with_ymd(2021, 1, 1).build().unwrap();
        let to = d2dt().with_ymd(2022, 1, 10).build().unwrap();

        let r = cc.forward_rate(&from, &to).unwrap();

        assert_eq!(r, Act365fRate::from_rate(0.03));
        cc.components
            .values_mut()
            .for_each(|c| c.curve.checkpoint());
    }

    #[test]
    fn test_composite_curve_src_get() {
        let c = MockCSrc::with_call_count(&CallCount {
            get: Some(2),
            ..CallCount::zero()
        });
        let w = MockWSrc::with_call_count(&CallCount {
            get: Some(1),
            ..CallCount::zero()
        });
        let mut src = CompositeCurveSrc::new(c, w);
        let req = "USD";

        let r = src.get(req).unwrap();

        assert_eq!(r.components.len(), 2);
        assert_eq!(r.components["SOFR"].weight, -1.0);
        assert_eq!(
            r.components["SOFR"].curve.rate,
            Act365fRate::from_rate(0.01)
        );
        assert_eq!(r.components["FF_SPREAD"].weight, 2.0);
        assert_eq!(
            r.components["FF_SPREAD"].curve.rate,
            Act365fRate::from_rate(0.005)
        );
        src.crv.checkpoint();
        src.weight.checkpoint();
    }

    #[test]
    fn test_composite_curve_src_etag() {
        let c = MockCSrc::with_call_count(&CallCount {
            etag: Some(2),
            ..CallCount::zero()
        });
        let w = MockWSrc::with_call_count(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let mut src = CompositeCurveSrc::new(c, w);
        let req = "USD";
        let r = src.etag(req).unwrap();
        src.crv.checkpoint();
        src.weight.checkpoint();
        src.crv.setup(&CallCount {
            etag: Some(2),
            ..CallCount::zero()
        });
        src.weight.setup(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });

        let r2 = src.etag(req).unwrap();
        src.crv.checkpoint();
        src.weight.checkpoint();

        src.crv
            .expect_etag()
            .times(2)
            .returning(|s| Ok(format!("etag-{}", s.chars().rev().collect::<String>())));
        src.weight.setup(&CallCount {
            get_with_etag: Some(1),
            ..CallCount::zero()
        });
        let r3 = src.etag(req).unwrap();

        assert_eq!(r, r2); // without any change, etag should be same
        assert_ne!(r, r3); // with change, etag should be different
        src.crv.checkpoint();
        src.weight.checkpoint();
    }
}
