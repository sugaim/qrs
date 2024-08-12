use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};

use super::{
    adjust::YieldCurveAdj,
    composite::{Adjusted, Joint, Weighted},
    YieldCurve,
};

// -----------------------------------------------------------------------------
// Curve
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Curve<C, Adj = ()> {
    Atom(C),
    Adjusted(Adjusted<Box<Self>, Adj>),
    Joint(Joint<Box<Self>>),
    Weighted(Weighted<Box<Self>>),
}

impl<C: YieldCurve, Adj: YieldCurveAdj<C::Value>> YieldCurve for Curve<C, Adj> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        match self {
            Curve::Atom(curve) => curve.forward_rate(from, to),
            Curve::Adjusted(adj) => adj.forward_rate(from, to),
            Curve::Joint(joint) => joint.forward_rate(from, to),
            Curve::Weighted(comp) => comp.forward_rate(from, to),
        }
    }
}

// -----------------------------------------------------------------------------
// CurveReq
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum CurveReq<Adj> {
    Atom { name: String },
    Adjusted(Adjusted<Box<Self>, Adj>),
    Joint(Joint<Box<Self>>),
    Weighted(Weighted<Box<Self>>),
}

// -----------------------------------------------------------------------------
// CurveSrc
// CurveSrcInduce
// -----------------------------------------------------------------------------
pub trait CurveSrc<Adj> {
    type Curve: YieldCurve;

    fn get_curve(&self, req: CurveReq<Adj>) -> anyhow::Result<Self::Curve>;
}

pub trait CurveSrcInduce {
    type AtomCurve: YieldCurve;

    fn get_curve_atom(&self, name: &str) -> anyhow::Result<Self::AtomCurve>;
}

impl<S: CurveSrcInduce, Adj> CurveSrc<Adj> for S
where
    Adj: YieldCurveAdj<<S::AtomCurve as YieldCurve>::Value>,
{
    type Curve = Curve<S::AtomCurve, Adj>;

    fn get_curve(&self, req: CurveReq<Adj>) -> anyhow::Result<Self::Curve> {
        match req {
            CurveReq::Atom { name } => self.get_curve_atom(&name).map(Curve::Atom),
            CurveReq::Adjusted(Adjusted { curve, adjustment }) => Ok(Curve::Adjusted(Adjusted {
                curve: Box::new(self.get_curve(*curve)?),
                adjustment,
            })),
            CurveReq::Joint(Joint {
                switch_point,
                short,
                long,
            }) => Ok(Curve::Joint(Joint {
                switch_point: switch_point.clone(),
                short: Box::new(self.get_curve(*short)?),
                long: Box::new(self.get_curve(*long)?),
            })),
            CurveReq::Weighted(Weighted { components }) => Ok(Curve::Weighted(Weighted {
                components: components
                    .into_iter()
                    .map(|c| self.get_curve(*c.0).map(|curve| (Box::new(curve), c.1)))
                    .collect::<anyhow::Result<_>>()?,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::curve::{adjust::Bump, atom::Flat};
    use qmath::num::Log;

    use super::*;

    struct MockCSrc {
        unit: f64,
    }

    impl CurveSrcInduce for MockCSrc {
        type AtomCurve = Flat<f64>;

        fn get_curve_atom(&self, req: &str) -> anyhow::Result<Self::AtomCurve> {
            let num: f64 = req.parse()?;
            Ok(Flat {
                rate: self.unit * num,
            })
        }
    }

    #[test]
    fn test_discount() {
        let crv = Flat { rate: 0.05 };
        let from = "2021-01-01T00:00:00Z".parse().unwrap();
        let to = "2021-01-31T00:00:00Z".parse().unwrap();

        let df = crv.discount(&from, &to).unwrap();

        approx::assert_abs_diff_eq!(df.log(), -0.05 * 30.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_get_atom() {
        let src = MockCSrc { unit: 0.02 };
        let req = CurveReq::<()>::Atom {
            name: "2".to_string(),
        };

        let curve = src.get_curve(req).unwrap();

        assert_eq!(curve, Curve::Atom(Flat { rate: 0.04 }));
    }

    #[test]
    fn test_get_adjusted() {
        let src = MockCSrc { unit: 0.02 };
        let req = CurveReq::<Bump<f64>>::Adjusted(Adjusted {
            curve: Box::new(CurveReq::Atom {
                name: "2".to_string(),
            }),
            adjustment: vec![Bump::with_flat(0.03)],
        });

        let curve = src.get_curve(req).unwrap();

        assert_eq!(
            curve,
            Curve::Adjusted(Adjusted {
                curve: Box::new(Curve::Atom(Flat { rate: 0.04 })),
                adjustment: vec![Bump::with_flat(0.03)],
            })
        );
    }

    #[test]
    fn test_get_joint() {
        let src = MockCSrc { unit: 0.02 };
        let req = CurveReq::<()>::Joint(Joint {
            switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
            short: Box::new(CurveReq::Atom {
                name: "2".to_string(),
            }),
            long: Box::new(CurveReq::Atom {
                name: "3".to_string(),
            }),
        });

        let curve = src.get_curve(req).unwrap();

        assert_eq!(
            curve,
            Curve::Joint(Joint {
                switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                short: Box::new(Curve::Atom(Flat { rate: 0.04 })),
                long: Box::new(Curve::Atom(Flat { rate: 0.06 })),
            })
        );
    }

    #[test]
    fn test_get_weighted() {
        let src = MockCSrc { unit: 0.02 };
        let req = CurveReq::<()>::Weighted(Weighted {
            components: vec![
                (
                    Box::new(CurveReq::Atom {
                        name: "2".to_string(),
                    }),
                    0.03,
                ),
                (
                    Box::new(CurveReq::Atom {
                        name: "3".to_string(),
                    }),
                    0.04,
                ),
            ],
        });

        let curve = src.get_curve(req).unwrap();

        assert_eq!(
            curve,
            Curve::Weighted(Weighted {
                components: vec![
                    (Box::new(Curve::Atom(Flat { rate: 0.04 })), 0.03,),
                    (Box::new(Curve::Atom(Flat { rate: 0.06 })), 0.04,),
                ]
            })
        );
    }

    #[test]
    fn test_get_nested() {
        let src = MockCSrc { unit: 0.02 };
        let req = CurveReq::<Bump<f64>>::Weighted(Weighted {
            components: vec![
                (
                    Box::new(CurveReq::Joint(Joint {
                        switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                        short: Box::new(CurveReq::Atom {
                            name: "2".to_string(),
                        }),
                        long: Box::new(CurveReq::Adjusted(Adjusted {
                            curve: Box::new(CurveReq::Atom {
                                name: "3".to_string(),
                            }),
                            adjustment: vec![Bump::with_flat(0.03), Bump::with_flat(0.04)],
                        })),
                    })),
                    0.03,
                ),
                (
                    Box::new(CurveReq::Atom {
                        name: "4".to_string(),
                    }),
                    0.04,
                ),
            ],
        });

        let curve = src.get_curve(req).unwrap();

        assert_eq!(
            curve,
            Curve::Weighted(Weighted {
                components: vec![
                    (
                        Box::new(Curve::Joint(Joint {
                            switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                            short: Box::new(Curve::Atom(Flat { rate: 0.04 })),
                            long: Box::new(Curve::Adjusted(Adjusted {
                                curve: Box::new(Curve::Atom(Flat { rate: 0.06 })),
                                adjustment: vec![Bump::with_flat(0.03), Bump::with_flat(0.04)],
                            })),
                        })),
                        0.03,
                    ),
                    (Box::new(Curve::Atom(Flat { rate: 0.08 })), 0.04,),
                ]
            })
        );
    }
}
