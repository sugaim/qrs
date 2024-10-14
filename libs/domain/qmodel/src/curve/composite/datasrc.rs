use crate::curve::{adjust::YieldCurveAdj, CurveSrc, YieldCurve};

use super::{Adjusted, Composite, Joint, Weighted};

// -----------------------------------------------------------------------------
// CompositeReq
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompositeReq<Adj> {
    Atom { name: String },
    Adjusted(Adjusted<Box<Self>, Adj>),
    Joint(Joint<Box<Self>>),
    Weighted(Weighted<Box<Self>>),
}

// -----------------------------------------------------------------------------
// CompositeSrc
// -----------------------------------------------------------------------------
pub trait CompositeSrc<Adj>: CurveSrc {
    fn get_composite_curve(
        &self,
        req: CompositeReq<Adj>,
    ) -> anyhow::Result<Composite<Self::Curve, Adj>> {
        match req {
            CompositeReq::Atom { name } => self.get_curve(&name).map(Composite::Atom),
            CompositeReq::Adjusted(Adjusted { curve, adjustment }) => {
                Ok(Composite::Adjusted(Adjusted {
                    curve: Box::new(self.get_composite_curve(*curve)?),
                    adjustment,
                }))
            }
            CompositeReq::Joint(Joint {
                switch_point,
                short,
                long,
            }) => Ok(Composite::Joint(Joint {
                switch_point: switch_point.clone(),
                short: Box::new(self.get_composite_curve(*short)?),
                long: Box::new(self.get_composite_curve(*long)?),
            })),
            CompositeReq::Weighted(Weighted { components }) => Ok(Composite::Weighted(Weighted {
                components: components
                    .into_iter()
                    .map(|c| {
                        self.get_composite_curve(*c.0)
                            .map(|curve| (Box::new(curve), c.1))
                    })
                    .collect::<anyhow::Result<_>>()?,
            })),
        }
    }
}

impl<S: CurveSrc, Adj: YieldCurveAdj<<S::Curve as YieldCurve>::Value>> CompositeSrc<Adj> for S {}

#[cfg(test)]
mod tests {
    use crate::curve::{adjust::Bump, atom::Flat, YieldCurve};
    use qmath::num::Log;

    use super::*;

    struct MockCSrc {
        unit: f64,
    }

    impl CurveSrc for MockCSrc {
        type Curve = Flat<f64>;

        fn get_curve(&self, req: &str) -> anyhow::Result<Self::Curve> {
            let num: f64 = req.parse()?;
            Ok(Flat {
                rate: (self.unit * num).into(),
            })
        }
    }

    #[test]
    fn test_discount() {
        let crv = Flat { rate: 0.05.into() };
        let from = "2021-01-01T00:00:00Z".parse().unwrap();
        let to = "2021-01-31T00:00:00Z".parse().unwrap();

        let df = crv.discount(&from, &to).unwrap();

        approx::assert_abs_diff_eq!(df.log(), -0.05 * 30.0 / 365.0, epsilon = 1e-10);
    }

    #[test]
    fn test_get_atom() {
        let src = MockCSrc { unit: 0.02 };
        let req = CompositeReq::<()>::Atom {
            name: "2".to_string(),
        };

        let curve = src.get_composite_curve(req).unwrap();

        assert_eq!(curve, Composite::Atom(Flat { rate: 0.04.into() }));
    }

    #[test]
    fn test_get_adjusted() {
        let src = MockCSrc { unit: 0.02 };
        let req = CompositeReq::<Bump<Flat<f64>>>::Adjusted(Adjusted {
            curve: Box::new(CompositeReq::Atom {
                name: "2".to_string(),
            }),
            adjustment: vec![Bump {
                adjuster: Flat { rate: 0.03.into() },
            }],
        });

        let curve = src.get_composite_curve(req).unwrap();

        assert_eq!(
            curve,
            Composite::Adjusted(Adjusted {
                curve: Box::new(Composite::Atom(Flat { rate: 0.04.into() })),
                adjustment: vec![Bump {
                    adjuster: Flat { rate: 0.03.into() }
                }],
            })
        );
    }

    #[test]
    fn test_get_joint() {
        let src = MockCSrc { unit: 0.02 };
        let req = CompositeReq::<()>::Joint(Joint {
            switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
            short: Box::new(CompositeReq::Atom {
                name: "2".to_string(),
            }),
            long: Box::new(CompositeReq::Atom {
                name: "3".to_string(),
            }),
        });

        let curve = src.get_composite_curve(req).unwrap();

        assert_eq!(
            curve,
            Composite::Joint(Joint {
                switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                short: Box::new(Composite::Atom(Flat { rate: 0.04.into() })),
                long: Box::new(Composite::Atom(Flat { rate: 0.06.into() })),
            })
        );
    }

    #[test]
    fn test_get_weighted() {
        let src = MockCSrc { unit: 0.02 };
        let req = CompositeReq::<()>::Weighted(Weighted {
            components: vec![
                (
                    Box::new(CompositeReq::Atom {
                        name: "2".to_string(),
                    }),
                    0.03,
                ),
                (
                    Box::new(CompositeReq::Atom {
                        name: "3".to_string(),
                    }),
                    0.04,
                ),
            ],
        });

        let curve = src.get_composite_curve(req).unwrap();

        assert_eq!(
            curve,
            Composite::Weighted(Weighted {
                components: vec![
                    (Box::new(Composite::Atom(Flat { rate: 0.04.into() })), 0.03,),
                    (Box::new(Composite::Atom(Flat { rate: 0.06.into() })), 0.04,),
                ]
            })
        );
    }

    #[test]
    fn test_get_nested() {
        let src = MockCSrc { unit: 0.02 };
        let req = CompositeReq::<Bump<Flat<f64>>>::Weighted(Weighted {
            components: vec![
                (
                    Box::new(CompositeReq::Joint(Joint {
                        switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                        short: Box::new(CompositeReq::Atom {
                            name: "2".to_string(),
                        }),
                        long: Box::new(CompositeReq::Adjusted(Adjusted {
                            curve: Box::new(CompositeReq::Atom {
                                name: "3".to_string(),
                            }),
                            adjustment: vec![
                                Bump {
                                    adjuster: Flat { rate: 0.03.into() },
                                },
                                Bump {
                                    adjuster: Flat { rate: 0.04.into() },
                                },
                            ],
                        })),
                    })),
                    0.03,
                ),
                (
                    Box::new(CompositeReq::Atom {
                        name: "4".to_string(),
                    }),
                    0.04,
                ),
            ],
        });

        let curve = src.get_composite_curve(req).unwrap();

        assert_eq!(
            curve,
            Composite::Weighted(Weighted {
                components: vec![
                    (
                        Box::new(Composite::Joint(Joint {
                            switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
                            short: Box::new(Composite::Atom(Flat { rate: 0.04.into() })),
                            long: Box::new(Composite::Adjusted(Adjusted {
                                curve: Box::new(Composite::Atom(Flat { rate: 0.06.into() })),
                                adjustment: vec![
                                    Bump {
                                        adjuster: Flat { rate: 0.03.into() },
                                    },
                                    Bump {
                                        adjuster: Flat { rate: 0.04.into() },
                                    },
                                ],
                            })),
                        })),
                        0.03,
                    ),
                    (Box::new(Composite::Atom(Flat { rate: 0.08.into() })), 0.04,),
                ]
            })
        );
    }
}
