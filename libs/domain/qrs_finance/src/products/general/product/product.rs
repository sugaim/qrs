use std::{
    collections::HashMap,
    hash::Hash,
    str::FromStr,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Context};
use qrs_chrono::{Calendar, CalendarSymbol, DateTime, DateToDateTime, DateWithTag};
use qrs_collections::RequireMinSize;
use qrs_datasrc::DataSrc;
use qrs_math::{num::Real, rounding::Rounding};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    daycount::{DayCount, DayCountSymbol},
    products::{
        general::{
            cashflow::{
                Cashflow, CashflowFixing, CashflowWithFixing, CouponBase, FixedCoupon,
                OvernightIndexCoupon, OvernightIndexFixing,
            },
            constant::Constant,
            core::{ComponentCategory, ComponentKey, ValueOrId, VariableTypes, WithId},
            leg::{Leg, StraightLeg},
            market::{Market, OvernightRate},
            process::{ConstantFloat, DeterministicFloat, MarketRef, Process},
        },
        in_arrears::{InArrears, SpreadExclusiveCompounding, StraightCompounding},
        Collateral,
    },
    Money,
};

use super::{
    data::{ProductData, _ComponentDependency},
    VariableTypesForData,
};

// -----------------------------------------------------------------------------
// Product
//
#[derive(Debug, Clone, PartialEq)]
pub struct Product<Ts: VariableTypes = DefaultVariableTypes> {
    dep: _ComponentDependency,
    col: Collateral,
    mkts: HashMap<String, Ts::MarketRef>,
    procs: HashMap<String, Ts::ProcessRef>,
    cfs: HashMap<String, Ts::CashflowRef>,
    legs: HashMap<String, Ts::LegRef>,
}

//
// methods
//
impl<Ts: VariableTypes> Product<Ts> {
    #[inline]
    pub fn collateral(&self) -> &Collateral {
        &self.col
    }

    #[inline]
    pub fn markets(&self) -> &HashMap<String, Ts::MarketRef> {
        &self.mkts
    }

    #[inline]
    pub fn processes(&self) -> &HashMap<String, Ts::ProcessRef> {
        &self.procs
    }

    #[inline]
    pub fn cashflows(&self) -> &HashMap<String, Ts::CashflowRef> {
        &self.cfs
    }

    #[inline]
    pub fn legs(&self) -> &HashMap<String, Ts::LegRef> {
        &self.legs
    }
}

// -----------------------------------------------------------------------------
// ConvertProduct
//
pub trait ConvertProduct<From: VariableTypes, To: VariableTypes> {
    fn initialize(&mut self);
    fn post_validation(&self, result: &Product<To>) -> anyhow::Result<()>;

    fn convert_mkt(&self, cmp: From::MarketRef) -> anyhow::Result<To::MarketRef>;

    fn convert_proc(
        &self,
        cmp: From::ProcessRef,
        mkts: &HashMap<String, To::MarketRef>,
    ) -> anyhow::Result<To::ProcessRef>;

    fn convert_cf(
        &self,
        cmp: From::CashflowRef,
        mkts: &HashMap<String, To::MarketRef>,
        procs: &HashMap<String, To::ProcessRef>,
    ) -> anyhow::Result<To::CashflowRef>;

    fn convert_leg(
        &self,
        cmp: From::LegRef,
        procs: &HashMap<String, To::ProcessRef>,
        cfs: &HashMap<String, To::CashflowRef>,
    ) -> anyhow::Result<To::LegRef>;

    fn convert_product(&self, mut product: Product<From>) -> anyhow::Result<Product<To>> {
        let mut mkts = HashMap::new();
        let mut procs = HashMap::new();
        let mut cfs = HashMap::new();
        let mut legs = HashMap::new();

        let dep = product.dep;
        for ComponentKey { cat, name } in dep.topological_sorted().iter().rev() {
            match cat {
                ComponentCategory::Constant => {}
                ComponentCategory::Market => {
                    if let Some(cmp) = product.mkts.remove(name) {
                        let mkt = self.convert_mkt(cmp)?;
                        mkts.insert(name.clone(), mkt);
                    }
                }
                ComponentCategory::Process => {
                    if let Some(cmp) = product.procs.remove(name) {
                        let proc = self.convert_proc(cmp, &mkts)?;
                        procs.insert(name.clone(), proc);
                    }
                }
                ComponentCategory::Cashflow => {
                    if let Some(cmp) = product.cfs.remove(name) {
                        let cf = self.convert_cf(cmp, &mkts, &procs)?;
                        cfs.insert(name.clone(), cf);
                    }
                }
                ComponentCategory::Leg => {
                    if let Some(cmp) = product.legs.remove(name) {
                        let leg = self.convert_leg(cmp, &procs, &cfs)?;
                        legs.insert(name.clone(), leg);
                    }
                }
            }
        }
        let res = Product {
            dep,
            col: product.col,
            mkts,
            procs,
            cfs,
            legs,
        };
        self.post_validation(&res)?;
        Ok(res)
    }
}

// -----------------------------------------------------------------------------
// BuildProduct
//
pub trait BuildProduct<V>: Sized {
    type Variables: VariableTypes;

    fn initialize(&mut self);
    fn post_validation(&self, result: &Product<Self::Variables>) -> anyhow::Result<()>;

    // market
    fn parse_mkt_overnight_rate(
        &self,
        cmp: &OvernightRate,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::MarketRef>;

    // process
    fn parse_proc_constant_float(
        &self,
        cmp: &ConstantFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_market_ref(
        &self,
        cmp: &MarketRef<VariableTypesForData<V>>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    // cashflow
    fn parse_cf_fixed_coupon(
        &self,
        cmp: &FixedCoupon<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    fn parse_cf_overnight_index_coupon(
        &self,
        cmp: &OvernightIndexCoupon<VariableTypesForData<V>>,
        fixing: Option<&OvernightIndexFixing>,
        consts: &HashMap<String, Constant>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    // leg
    fn parse_leg_straight(
        &self,
        leg: &StraightLeg<VariableTypesForData<V>>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::LegRef>;

    // build
    fn build(mut self, data: &ProductData<V>) -> anyhow::Result<Product<Self::Variables>> {
        self.initialize();
        let dep = data.contract._dependency()?;
        let mut mkts = HashMap::new();
        let mut procs = HashMap::new();
        let mut cfs = HashMap::new();
        let mut legs = HashMap::new();
        for ComponentKey { cat, name } in dep.topological_sorted().iter().rev() {
            match cat {
                ComponentCategory::Constant => {}
                ComponentCategory::Market => {
                    let cmp = data.contract.markets.get(name).unwrap();
                    let mkt = match cmp {
                        Market::OvernightRate(cmp) => {
                            self.parse_mkt_overnight_rate(cmp, &data.contract.constants)?
                        }
                    };
                    mkts.insert(name.clone(), mkt);
                }
                ComponentCategory::Process => {
                    let cmp = data.contract.processes.get(name).unwrap();
                    let proc = match cmp {
                        Process::ConstantFloat(cmp) => {
                            self.parse_proc_constant_float(cmp, &data.contract.constants)?
                        }
                        Process::DeterministicFloat(cmp) => {
                            self.parse_proc_deternministic_float(cmp, &data.contract.constants)?
                        }
                        Process::MarketRef(cmp) => self.parse_proc_market_ref(cmp, &mkts)?,
                    };
                    procs.insert(name.clone(), proc);
                }
                ComponentCategory::Cashflow => {
                    let cmp = data.contract.cashflows.get(name).unwrap();
                    let fixing = data.fixing.cashflows.get(name);
                    let cf = match (cmp, fixing) {
                        (Cashflow::FixedCoupon(cmp), None) => {
                            self.parse_cf_fixed_coupon(cmp, &data.contract.constants)?
                        }
                        (Cashflow::OvernightIndexCoupon(cmp), None) => self
                            .parse_cf_overnight_index_coupon(
                                cmp,
                                None,
                                &data.contract.constants,
                                &mkts,
                            )?,
                        (
                            Cashflow::OvernightIndexCoupon(cmp),
                            Some(CashflowFixing::OvernightIndexCoupon(fixing)),
                        ) => self.parse_cf_overnight_index_coupon(
                            cmp,
                            Some(fixing),
                            &data.contract.constants,
                            &mkts,
                        )?,
                        _ => bail!("unsupported cashflow type"),
                    };
                    cfs.insert(name.clone(), cf);
                }
                ComponentCategory::Leg => {
                    let cmp = data.contract.legs.get(name).unwrap();
                    let leg = match cmp {
                        Leg::Straight(cmp) => self.parse_leg_straight(cmp, &cfs)?,
                    };
                    legs.insert(name.clone(), leg);
                }
            }
        }
        let res = Product {
            dep,
            col: data.contract.collateral.clone(),
            mkts,
            procs,
            cfs,
            legs,
        };
        self.post_validation(&res)?;
        Ok(res)
    }
}

// -----------------------------------------------------------------------------
// DefaultVariableTypes
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema)]
pub struct DefaultVariableTypes<V = f64>(std::marker::PhantomData<V>);

//
// methods
//
impl<V> VariableTypes for DefaultVariableTypes<V> {
    type Boolean = bool;
    type Integer = i64;
    type Number = V;

    type DateTime = DateTime;
    type Calendar = Calendar;
    type DayCount = DayCount;

    type MarketRef = Arc<Market>;
    type ProcessRef = Arc<Process<Self>>;
    type CashflowRef = Arc<CashflowWithFixing<Self>>;
    type LegRef = Arc<Leg<Self>>;

    type InArrearsConvention = Arc<InArrears<DayCount, Calendar>>;
    type Rounding = Rounding;
}

// -----------------------------------------------------------------------------
//  DefaultProductBuilder
//
#[derive(Debug)]
pub struct DefaultProductBuilder<DayCnt = (), Cal = (), TimeCut = ()> {
    daycnt_src: DayCnt,
    cal_src: Cal,
    time_cut: TimeCut,
    conv: Mutex<HashMap<String, Arc<InArrears<DayCount, Calendar>>>>,
    rounding: Mutex<HashMap<String, Rounding>>,
}

//
// construction
//
impl DefaultProductBuilder {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

impl<DayCnt, Cal, TimeCut> Clone for DefaultProductBuilder<DayCnt, Cal, TimeCut>
where
    DayCnt: Clone,
    Cal: Clone,
    TimeCut: Clone,
{
    fn clone(&self) -> Self {
        Self {
            daycnt_src: self.daycnt_src.clone(),
            cal_src: self.cal_src.clone(),
            time_cut: self.time_cut.clone(),
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl Default for DefaultProductBuilder {
    fn default() -> Self {
        Self {
            daycnt_src: (),
            cal_src: (),
            time_cut: (),
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl<Cal, TimeCut> DefaultProductBuilder<(), Cal, TimeCut> {
    #[inline]
    pub fn with_daycount_src<DayCnt>(
        self,
        daycnt_src: DayCnt,
    ) -> DefaultProductBuilder<DayCnt, Cal, TimeCut>
    where
        DayCnt: DataSrc<str, Output = DayCount>,
    {
        DefaultProductBuilder {
            daycnt_src,
            cal_src: self.cal_src,
            time_cut: self.time_cut,
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl<DayCnt, TimeCut> DefaultProductBuilder<DayCnt, (), TimeCut> {
    #[inline]
    pub fn with_calendar_src<Cal>(self, cal_src: Cal) -> DefaultProductBuilder<DayCnt, Cal, TimeCut>
    where
        Cal: DataSrc<CalendarSymbol, Output = Calendar>,
    {
        DefaultProductBuilder {
            daycnt_src: self.daycnt_src,
            cal_src,
            time_cut: self.time_cut,
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl<DayCnt, Cal> DefaultProductBuilder<DayCnt, Cal, ()> {
    #[inline]
    pub fn with_time_cut<TimeCut>(
        self,
        time_cut: TimeCut,
    ) -> DefaultProductBuilder<DayCnt, Cal, TimeCut>
    where
        TimeCut: DataSrc<str, Output = DateToDateTime>,
    {
        DefaultProductBuilder {
            daycnt_src: self.daycnt_src,
            cal_src: self.cal_src,
            time_cut,
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

//
// methods
//
impl<D, C, T, V> BuildProduct<V> for DefaultProductBuilder<D, C, T>
where
    D: DataSrc<DayCountSymbol, Output = DayCount>,
    C: DataSrc<CalendarSymbol, Output = Calendar>,
    T: DataSrc<str, Output = DateToDateTime>,
    V: Real,
{
    type Variables = DefaultVariableTypes<V>;

    #[inline]
    fn initialize(&mut self) {
        self.conv.lock().unwrap().clear();
    }
    #[inline]
    fn post_validation(&self, _: &Product<Self::Variables>) -> anyhow::Result<()> {
        Ok(())
    }

    // market
    #[inline]
    fn parse_mkt_overnight_rate(
        &self,
        cmp: &OvernightRate,
        _: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::MarketRef> {
        Ok(Arc::new(Market::OvernightRate(cmp.clone())))
    }

    // process
    fn parse_proc_constant_float(
        &self,
        cmp: &ConstantFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        const NAME: &str = "constant_float";
        let values = cmp.values.iter();
        let values = values.map(|v| self._unwrap_float(v, consts, NAME));
        let values = values
            .collect::<anyhow::Result<Vec<_>>>()?
            .require_min_size()?;
        Ok(Arc::new(Process::ConstantFloat(ConstantFloat { values })))
    }

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        const NAME: &str = "deterministic_float";
        let mut series = Vec::default();
        for ser in cmp.series.iter() {
            let ts = ser.iter().map(|(dt, v)| {
                let dt = self._unwrap_dt(dt, consts, NAME)?;
                let v = self._unwrap_float(v, consts, NAME)?;
                Ok((dt, v))
            });
            let ts = ts.collect::<anyhow::Result<HashMap<_, _>>>()?;
            series.push(ts.require_min_size()?);
        }
        let series = series.require_min_size()?;
        Ok(Arc::new(Process::DeterministicFloat(DeterministicFloat {
            series,
        })))
    }

    fn parse_proc_market_ref(
        &self,
        cmp: &MarketRef<VariableTypesForData<V>>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        const NAME: &str = "market_ref";
        let refs = cmp.refs.iter();
        let refs = refs.map(|m| {
            let id = &m.id;
            mkts.get(id)
                .map(|m| WithId {
                    id: id.clone(),
                    value: m.clone(),
                })
                .ok_or_else(|| anyhow!("market `{id}` is not found which is required by {NAME}"))
        });
        let refs = refs
            .collect::<anyhow::Result<Vec<_>>>()?
            .require_min_size()?;
        Ok(Arc::new(Process::MarketRef(MarketRef { refs })))
    }

    //
    fn parse_cf_fixed_coupon(
        &self,
        cmp: &FixedCoupon<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        const NAME: &str = "fixed_coupon";
        Ok(Arc::new(CashflowWithFixing::FixedCoupon(FixedCoupon {
            base: self._unwrap_cpnbase(&cmp.base, consts, NAME)?,
            rate: self._unwrap_float(&cmp.rate, consts, NAME)?,
            accrual: self._unwrap_dcnt(&cmp.accrual, consts, NAME)?,
            rounding: cmp
                .rounding
                .as_ref()
                .map(|r| self._unwrap_rounding(r, consts, NAME))
                .transpose()?,
        })))
    }

    fn parse_cf_overnight_index_coupon(
        &self,
        cmp: &OvernightIndexCoupon<VariableTypesForData<V>>,
        fixing: Option<&OvernightIndexFixing>,
        consts: &HashMap<String, Constant>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        const NAME: &str = "overnight_index_coupon";
        let ref_rate = mkts.get(&cmp.reference_rate.id).ok_or_else(|| {
            anyhow!(
                "market `{}` is not found which is required by {NAME}",
                cmp.reference_rate.id
            )
        })?;
        if !matches!(ref_rate.as_ref(), Market::OvernightRate(_)) {
            bail!(
                "{NAME} requires market `{}` is an overnight rate",
                cmp.reference_rate.id
            );
        }
        Ok(Arc::new(CashflowWithFixing::OvernightIndexCoupon(
            OvernightIndexCoupon {
                base: self._unwrap_cpnbase(&cmp.base, consts, NAME)?,
                convention: self._unwrap_inarrears(&cmp.convention, consts, NAME)?,
                reference_rate: WithId {
                    id: cmp.reference_rate.id.clone(),
                    value: ref_rate.clone(),
                },
                gearing: cmp
                    .gearing
                    .as_ref()
                    .map(|v| self._unwrap_float(v, consts, NAME))
                    .transpose()?,
                spread: cmp
                    .spread
                    .as_ref()
                    .map(|v| self._unwrap_float(v, consts, NAME))
                    .transpose()?,
                rounding: cmp
                    .rounding
                    .as_ref()
                    .map(|r| self._unwrap_rounding(r, consts, NAME))
                    .transpose()?,
            },
            fixing.cloned(),
        )))
    }

    // leg
    fn parse_leg_straight(
        &self,
        leg: &StraightLeg<VariableTypesForData<V>>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::LegRef> {
        const NAME: &str = "straight_leg";
        let cashflows = leg.cashflows.iter();
        let cashflows = cashflows.map(|cf| {
            let id = &cf.id;
            cfs.get(id)
                .map(|cf| WithId {
                    id: id.clone(),
                    value: cf.clone(),
                })
                .ok_or_else(|| anyhow!("cashflow `{id}` is not found which is required by {NAME}"))
        });
        let cashflows = cashflows.collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Arc::new(Leg::Straight(StraightLeg { cashflows })))
    }
}

impl<D, C, T> DefaultProductBuilder<D, C, T>
where
    D: DataSrc<DayCountSymbol, Output = DayCount>,
    C: DataSrc<CalendarSymbol, Output = Calendar>,
    T: DataSrc<str, Output = DateToDateTime>,
{
    fn _ctx_msg(&self, cmp: &str) -> String {
        format!("Parse {cmp} to convert `ProductData` to `Product`")
    }
    fn _unwrap_float<V: Real>(
        &self,
        v: &ValueOrId<V>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<V> {
        let id = match v {
            ValueOrId::Value(v) => return Ok(v.clone()),
            ValueOrId::Id(id) => id,
        };
        consts
            .get(id)
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Number(v) => Ok(V::nearest_base_float_of(*v).into()),
                _ => bail!("constant `{id}` is not a number."),
            })
    }

    fn _unwrap_dt(
        &self,
        v: &ValueOrId<DateWithTag>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<DateTime> {
        let id = match v {
            ValueOrId::Value(v) => {
                return v
                    .to_datetime(&self.time_cut)
                    .with_context(|| self._ctx_msg(required_by))
            }
            ValueOrId::Id(id) => id,
        };
        consts
            .get(id)
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::String(s) => DateWithTag::<String>::from_str(s).map_err(Into::into),
                _ => bail!("constant `{id}` is not a string, which is expected a `DateWithTag`."),
            })
            .and_then(|dt| {
                dt.to_datetime(&self.time_cut)
                    .with_context(|| self._ctx_msg(required_by))
            })
    }

    fn _unwrap_dcnt(
        &self,
        v: &ValueOrId<DayCountSymbol>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<DayCount> {
        let id = match v {
            ValueOrId::Value(v) => {
                return self
                    .daycnt_src
                    .get(v)
                    .with_context(|| self._ctx_msg(required_by))
            }
            ValueOrId::Id(id) => id,
        };
        consts
            .get(id)
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => DayCountSymbol::deserialize(v).map_err(Into::into),
                _ => {
                    bail!("constant `{id}` is not an object, which is expected a `DayCountSymbol`.")
                }
            })
            .and_then(|s| {
                self.daycnt_src
                    .get(&s)
                    .with_context(|| self._ctx_msg(required_by))
            })
    }

    fn _unwrap_rounding(
        &self,
        v: &ValueOrId<Rounding>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<Rounding> {
        let id = match v {
            ValueOrId::Value(v) => return Ok(*v),
            ValueOrId::Id(id) => id,
        };
        let mut cache = self.rounding.lock().unwrap();
        if let Some(res) = cache.get(id) {
            return Ok(*res);
        }
        let res = consts
            .get(id)
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => Rounding::deserialize(v)
                    .map_err(Into::<anyhow::Error>::into)
                    .context("Converting ProductData to Product"),
                _ => bail!("constant `{id}` is not an object, which is expected a `Rounding`."),
            })?;
        cache.insert(id.clone(), res);
        Ok(res)
    }

    fn _unwrap_cpnbase<V: Real>(
        &self,
        v: &CouponBase<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<CouponBase<DefaultVariableTypes<V>>> {
        let base = CouponBase {
            notional: Money {
                amount: self._unwrap_float(&v.notional.amount, consts, required_by)?,
                ccy: v.notional.ccy,
            },
            period_start: self._unwrap_dt(&v.period_start, consts, required_by)?,
            period_end: self._unwrap_dt(&v.period_end, consts, required_by)?,
            entitle: self._unwrap_dt(&v.entitle, consts, required_by)?,
            payment: self._unwrap_dt(&v.payment, consts, required_by)?,
            daycount: self._unwrap_dcnt(&v.daycount, consts, required_by)?,
        };
        Ok(base)
    }

    fn _unwrap_inarrears(
        &self,
        v: &ValueOrId<InArrears<DayCountSymbol, CalendarSymbol>>,
        consts: &HashMap<String, Constant>,
        required_by: &str,
    ) -> anyhow::Result<Arc<InArrears<DayCount, Calendar>>> {
        let parse = |v: &InArrears<DayCountSymbol, CalendarSymbol>| {
            use InArrears::*;
            let cal_src = &self.cal_src;
            let dcnt_src = &self.daycnt_src;
            let res = match v {
                Straight(v) => Straight(StraightCompounding {
                    calendar: cal_src.get(&v.calendar)?,
                    obsrate_daycount: dcnt_src.get(&v.obsrate_daycount)?,
                    overall_daycount: dcnt_src.get(&v.overall_daycount)?,
                    lockout: v.lockout,
                    lookback: v.lookback,
                    rounding: v.rounding,
                    zero_interest_rate_method: v.zero_interest_rate_method,
                }),
                SpreadExclusive(v) => SpreadExclusive(SpreadExclusiveCompounding {
                    calendar: cal_src.get(&v.calendar)?,
                    obsrate_daycount: dcnt_src.get(&v.obsrate_daycount)?,
                    overall_daycount: dcnt_src.get(&v.overall_daycount)?,
                    lockout: v.lockout,
                    lookback: v.lookback,
                    rounding: v.rounding,
                    zero_interest_rate_method: v.zero_interest_rate_method,
                }),
            };
            anyhow::Ok(Arc::new(res))
        };
        let id = match v {
            ValueOrId::Value(v) => return parse(v).context("Converting ProductData to Product"),
            ValueOrId::Id(id) => id,
        };
        let mut cache = self.conv.lock().unwrap();
        if let Some(res) = cache.get(id) {
            return Ok(res.clone());
        }
        let res = consts
            .get(id)
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => InArrears::<DayCountSymbol, CalendarSymbol>::deserialize(v)
                    .map_err(Into::<anyhow::Error>::into)
                    .context("Converting ProductData to Product"),
                _ => bail!("constant `{id}` is not an object, which is expected a `InArrears`."),
            })
            .and_then(|s| parse(&s).context("Converting ProductData to Product"))?;
        cache.insert(id.clone(), res.clone());
        Ok(res)
    }
}
