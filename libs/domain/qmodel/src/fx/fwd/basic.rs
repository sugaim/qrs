use qchrono::timepoint::DateTime;
use qfincore::{
    fxmkt::{FxSpotMkt, FxSpotMktSrc},
    CcyPair, FxRate,
};
use qmath::num::Real;
use qproduct::Collateral;

use crate::{
    curve::YieldCurve,
    ir::dcrv::{DCrv, DCrvSrc},
};

use super::{
    super::spot::{FxSpot, FxSpotSrc},
    FxFwd, FxFwdSrc,
};

// -----------------------------------------------------------------------------
// BasicFxFwd
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct BasicFxFwd<V> {
    base: DCrv<V>,
    quote: DCrv<V>,
    spot: FxSpot<V>,
    mkt: FxSpotMkt,
}

//
// methods
//
impl<V: Real> FxFwd for BasicFxFwd<V> {
    type Value = V;

    #[inline]
    fn fxspot(&self) -> FxSpot<Self::Value> {
        self.spot.clone()
    }

    #[inline]
    fn forward_of(
        &self,
        spot_rate: &Self::Value,
        spot_date: &DateTime,
        tgt: &DateTime,
    ) -> anyhow::Result<FxRate<Self::Value>> {
        let qdisc = self.quote.discount(spot_date, tgt)?;
        let bdisc = self.base.discount(spot_date, tgt)?;
        Ok(FxRate {
            pair: self.spot.rate.pair,
            value: bdisc / &qdisc * spot_rate,
        })
    }

    #[inline]
    fn fwdspot_of(
        &self,
        spot_rate: &Self::Value,
        spot_date: &DateTime,
        tgt: &DateTime,
    ) -> anyhow::Result<FxRate<V>> {
        let tgt = self.mkt.spot_datetime_of(tgt)?;
        self.forward_of(spot_rate, spot_date, &tgt)
    }
}

// -----------------------------------------------------------------------------
// BasicFxFwdSrcInduce
// -----------------------------------------------------------------------------
pub trait BasicFxFwdSrcInduce:
    DCrvSrc + FxSpotSrc<Value = <Self as DCrvSrc>::Value> + FxSpotMktSrc
{
}

impl<S: BasicFxFwdSrcInduce> FxFwdSrc for S {
    type FxFwd = BasicFxFwd<<Self as DCrvSrc>::Value>;

    #[inline]
    fn get_fxfwd(&self, req: &CcyPair) -> anyhow::Result<Self::FxFwd> {
        Ok(BasicFxFwd {
            base: self.get_dcrv(&req.base.clone(), &Collateral::Ccy(req.quote))?,
            quote: self.get_dcrv(&req.quote.clone(), &Collateral::Ccy(req.quote))?,
            spot: self.get_fxspot(req)?,
            mkt: self.get_fxspot_mkt(req)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use qchrono::calendar::Calendar;
    use qfincore::Ccy;
    use rstest::rstest;

    use crate::{
        curve::{
            adjust::Adj,
            atom::{Atom, Flat},
            composite::CompositeReq,
            CurveSrc,
        },
        ir::dcrv::ResolveDCrv,
    };

    use super::*;

    struct MockCalendarSrc;

    impl CurveSrc for MockCalendarSrc {
        type Curve = Arc<Atom<f64>>;

        fn get_curve(&self, name: &str) -> anyhow::Result<Self::Curve> {
            match name {
                "USD" => Ok(Arc::new(Atom::Flat(Flat { rate: 0.005 }))),
                "JPY" => Ok(Arc::new(Atom::Flat(Flat { rate: 0.001 }))),
                _ => Err(anyhow::anyhow!("Unknown curve: {}", name)),
            }
        }
    }
    impl ResolveDCrv for MockCalendarSrc {
        type Value = f64;

        fn resolve_dcrv(
            &self,
            ccy: &Ccy,
            _: &Collateral,
        ) -> anyhow::Result<CompositeReq<Adj<f64>>> {
            let name = ccy.to_string();
            Ok(CompositeReq::Atom { name })
        }
    }
    impl FxSpotMktSrc for MockCalendarSrc {
        fn get_fxspot_mkt(&self, _: &CcyPair) -> anyhow::Result<FxSpotMkt> {
            Ok(FxSpotMkt {
                spot_lag: 2,
                settle_cal: Calendar::blank(false),
            })
        }
    }
    impl FxSpotSrc for MockCalendarSrc {
        type Value = f64;

        fn get_fxspot(&self, _: &CcyPair) -> anyhow::Result<FxSpot<f64>> {
            Ok(FxSpot {
                spot_date: DateTime::from_str("2021-01-05T00:00:00Z")?,
                rate: FxRate {
                    pair: CcyPair {
                        base: Ccy::USD,
                        quote: Ccy::JPY,
                    },
                    value: 100.0,
                },
            })
        }
    }

    impl BasicFxFwdSrcInduce for MockCalendarSrc {}

    #[test]
    fn test_get_basic_fxfwd() {
        let src = MockCalendarSrc;
        let expected = BasicFxFwd {
            base: src.get_dcrv(&Ccy::USD, &Collateral::Ccy(Ccy::JPY)).unwrap(),
            quote: src.get_dcrv(&Ccy::JPY, &Collateral::Ccy(Ccy::JPY)).unwrap(),
            spot: FxSpot {
                spot_date: DateTime::from_str("2021-01-05T00:00:00Z").unwrap(),
                rate: FxRate {
                    pair: CcyPair {
                        base: Ccy::USD,
                        quote: Ccy::JPY,
                    },
                    value: 100.0,
                },
            },
            mkt: FxSpotMkt {
                spot_lag: 2,
                settle_cal: Calendar::blank(false),
            },
        };

        let res = src.get_fxfwd(&CcyPair {
            base: Ccy::USD,
            quote: Ccy::JPY,
        });

        assert_eq!(res.unwrap(), expected);
    }

    #[test]
    fn test_get_basic_fxfwd_err() {
        let src = MockCalendarSrc;
        let res = src.get_fxfwd(&CcyPair {
            base: Ccy::USD,
            quote: Ccy::EUR,
        });

        assert!(res.is_err());
    }

    #[test]
    fn test_basic_fxfwd_fxspot() {
        let src = MockCalendarSrc;
        let fxfwd = src
            .get_fxfwd(&CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY,
            })
            .unwrap();

        let spot = fxfwd.fxspot();
        assert_eq!(spot.rate.value, 100.0);
        assert_eq!(
            spot.rate.pair,
            CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY
            }
        );
        assert_eq!(
            spot.spot_date,
            DateTime::from_str("2021-01-05T00:00:00Z").unwrap()
        );
    }

    #[rstest]
    #[case(
        55.,
        "2021-01-05T00:00:00Z".parse().unwrap(),
        "2021-01-05T00:00:00Z".parse().unwrap(),
        55.
    )]
    #[case(
        60.,
        "2021-01-05T00:00:00Z".parse().unwrap(),
        "2021-01-01T00:00:00Z".parse().unwrap(),
        60. * f64::exp(0.004 * 4. / 365.)
    )]
    #[case(
        160.,
        "2021-01-10T00:00:00Z".parse().unwrap(),
        "2021-01-15T00:00:00Z".parse().unwrap(),
        160. * f64::exp(-0.004 * 5. / 365.)
    )]
    fn test_basic_fxfwd_foward_of(
        #[case] spot: f64,
        #[case] spot_dt: DateTime,
        #[case] tgt: DateTime,
        #[case] expected: f64,
    ) {
        let src = MockCalendarSrc;
        let fxfwd = src
            .get_fxfwd(&CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY,
            })
            .unwrap();

        let res = fxfwd.forward_of(&spot, &spot_dt, &tgt).unwrap();

        assert_eq!(
            res.pair,
            CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY
            }
        );
        approx::assert_abs_diff_eq!(res.value, expected, epsilon = 1e-10);
    }

    #[rstest]
    #[case(
        "2021-01-05T00:00:00Z".parse().unwrap(),
        100.
    )]
    #[case(
        "2021-01-01T00:00:00Z".parse().unwrap(),
        100. * f64::exp(0.004 * 4. / 365.)
    )]
    #[case(
        "2021-01-10T00:00:00Z".parse().unwrap(),
        100. * f64::exp(-0.004 * 5. / 365.)
    )]
    fn test_basic_fxfwd_forward(#[case] tgt: DateTime, #[case] expected: f64) {
        let src = MockCalendarSrc;
        let fxfwd = src
            .get_fxfwd(&CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY,
            })
            .unwrap();

        let res = fxfwd.forward(&tgt).unwrap();

        approx::assert_abs_diff_eq!(res.value, expected, epsilon = 1e-10);
    }

    #[rstest]
    #[case(
        55.,
        "2021-01-05T00:00:00Z".parse().unwrap(),
        "2021-01-05T00:00:00Z".parse().unwrap(),
        55. * f64::exp(-0.004 * 2. / 365.)
    )]
    #[case(
        60.,
        "2021-01-05T00:00:00Z".parse().unwrap(),
        "2021-01-01T00:00:00Z".parse().unwrap(),
        60.
    )]
    #[case(
        160.,
        "2021-01-10T00:00:00Z".parse().unwrap(),
        "2021-01-15T00:00:00Z".parse().unwrap(),
        160. * f64::exp(-0.004 * 9. / 365.)
    )]
    fn test_basic_fxfwd_fwdspot_of(
        #[case] spot: f64,
        #[case] spot_dt: DateTime,
        #[case] tgt: DateTime,
        #[case] expected: f64,
    ) {
        let src = MockCalendarSrc;
        let fxfwd = src
            .get_fxfwd(&CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY,
            })
            .unwrap();

        let res = fxfwd.fwdspot_of(&spot, &spot_dt, &tgt).unwrap();

        assert_eq!(
            res.pair,
            CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY
            }
        );
        approx::assert_abs_diff_eq!(res.value, expected, epsilon = 1e-10);
    }

    #[rstest]
    #[case(
        "2021-01-05T00:00:00Z".parse().unwrap(),
        100. * f64::exp(-0.004 * 2. / 365.)
    )]
    #[case(
        "2021-01-01T00:00:00Z".parse().unwrap(),
        100.
    )]
    #[case(
        "2021-01-15T00:00:00Z".parse().unwrap(),
        100. * f64::exp(-0.004 * 14. / 365.)
    )]
    fn test_basic_fxfwd_fwdspot(#[case] tgt: DateTime, #[case] expected: f64) {
        let src = MockCalendarSrc;
        let fxfwd = src
            .get_fxfwd(&CcyPair {
                base: Ccy::USD,
                quote: Ccy::JPY,
            })
            .unwrap();

        let res = fxfwd.fwdspot(&tgt).unwrap();

        approx::assert_abs_diff_eq!(res.value, expected, epsilon = 1e-10);
    }
}
