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
    products::general::{
        cashflow::{
            Cashflow, CashflowFixing, CashflowWithFixing, CouponBase, FixedCoupon,
            OvernightIndexCoupon, OvernightIndexFixing,
        },
        constant::Constant,
        core::{ComponentCategory, ComponentKey, ValueOrId, VariableTypes},
        leg::{Leg, StraightLeg},
        market::{Market, OvernightRate},
        process::{ConstantFloat, DeterministicFloat, MarketRef, Process},
    },
    products::in_arrears::{InArrears, SpreadExclusiveCompounding, StraightCompounding},
    Money,
};

use super::{data::ProductData, VariableTypesForData};

// -----------------------------------------------------------------------------
// Product
//
#[derive(Debug, Clone, PartialEq)]
pub struct Product<Ts: VariableTypes = VariableTypesExpanded> {
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
// BuildProduct
//
pub trait BuildProduct<V>: Sized {
    type Variables: VariableTypes;

    fn initialize(&mut self);

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
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_market_ref(
        &self,
        cmp: &MarketRef<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    // cashflow
    fn parse_cf_fixed_coupon(
        &self,
        cmp: &FixedCoupon<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    fn parse_cf_overnight_index_coupon(
        &self,
        cmp: &OvernightIndexCoupon<VariableTypesForData<V>>,
        fixing: Option<&OvernightIndexFixing>,
        consts: &HashMap<String, Constant>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    // leg
    fn parse_leg_straight(
        &self,
        leg: &StraightLeg<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
        legs: &HashMap<String, <Self::Variables as VariableTypes>::LegRef>,
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
                        Process::ConstantFloat(cmp) => self.parse_proc_constant_float(
                            cmp,
                            &data.contract.constants,
                            &mkts,
                            &procs,
                        )?,
                        Process::DeterministicFloat(cmp) => self.parse_proc_deternministic_float(
                            cmp,
                            &data.contract.constants,
                            &mkts,
                            &procs,
                        )?,
                        Process::MarketRef(cmp) => self.parse_proc_market_ref(
                            cmp,
                            &data.contract.constants,
                            &mkts,
                            &procs,
                        )?,
                    };
                    procs.insert(name.clone(), proc);
                }
                ComponentCategory::Cashflow => {
                    let cmp = data.contract.cashflows.get(name).unwrap();
                    let fixing = data.fixing.cashflows.get(name);
                    let cf = match (cmp, fixing) {
                        (Cashflow::FixedCoupon(cmp), None) => {
                            self.parse_cf_fixed_coupon(cmp, &data.contract.constants, &procs, &cfs)?
                        }
                        (Cashflow::OvernightIndexCoupon(cmp), None) => self
                            .parse_cf_overnight_index_coupon(
                                cmp,
                                None,
                                &data.contract.constants,
                                &procs,
                                &cfs,
                            )?,
                        (
                            Cashflow::OvernightIndexCoupon(cmp),
                            Some(CashflowFixing::OvernightIndexCoupon(fixing)),
                        ) => self.parse_cf_overnight_index_coupon(
                            cmp,
                            Some(fixing),
                            &data.contract.constants,
                            &procs,
                            &cfs,
                        )?,
                        _ => bail!("unsupported cashflow type"),
                    };
                    cfs.insert(name.clone(), cf);
                }
                ComponentCategory::Leg => {
                    let cmp = data.contract.legs.get(name).unwrap();
                    let leg = match cmp {
                        Leg::Straight(cmp) => self.parse_leg_straight(
                            cmp,
                            &data.contract.constants,
                            &procs,
                            &cfs,
                            &legs,
                        )?,
                    };
                    legs.insert(name.clone(), leg);
                }
            }
        }
        Ok(Product {
            mkts,
            procs,
            cfs,
            legs,
        })
    }
}

// -----------------------------------------------------------------------------
// VariableTypesExpanded
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, JsonSchema)]
pub struct VariableTypesExpanded<V = f64>(std::marker::PhantomData<V>);

//
// methods
//
impl<V> VariableTypes for VariableTypesExpanded<V> {
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
//  ProductBuilder
//
#[derive(Debug)]
pub struct ProductBuilder<DayCnt = (), Cal = (), TimeCut = ()> {
    daycnt_src: DayCnt,
    cal_src: Cal,
    time_cut: TimeCut,
    conv: Mutex<HashMap<String, Arc<InArrears<DayCount, Calendar>>>>,
    rounding: Mutex<HashMap<String, Rounding>>,
}

//
// construction
//
impl ProductBuilder {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

impl<DayCnt, Cal, TimeCut> Clone for ProductBuilder<DayCnt, Cal, TimeCut>
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

impl Default for ProductBuilder {
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

impl<Cal, TimeCut> ProductBuilder<(), Cal, TimeCut> {
    #[inline]
    pub fn with_daycount_src<DayCnt>(
        self,
        daycnt_src: DayCnt,
    ) -> ProductBuilder<DayCnt, Cal, TimeCut>
    where
        DayCnt: DataSrc<str, Output = DayCount>,
    {
        ProductBuilder {
            daycnt_src,
            cal_src: self.cal_src,
            time_cut: self.time_cut,
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl<DayCnt, TimeCut> ProductBuilder<DayCnt, (), TimeCut> {
    #[inline]
    pub fn with_calendar_src<Cal>(self, cal_src: Cal) -> ProductBuilder<DayCnt, Cal, TimeCut>
    where
        Cal: DataSrc<CalendarSymbol, Output = Calendar>,
    {
        ProductBuilder {
            daycnt_src: self.daycnt_src,
            cal_src,
            time_cut: self.time_cut,
            conv: Default::default(),
            rounding: Default::default(),
        }
    }
}

impl<DayCnt, Cal> ProductBuilder<DayCnt, Cal, ()> {
    #[inline]
    pub fn with_time_cut<TimeCut>(self, time_cut: TimeCut) -> ProductBuilder<DayCnt, Cal, TimeCut>
    where
        TimeCut: DataSrc<str, Output = DateToDateTime>,
    {
        ProductBuilder {
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
impl<D, C, T, V> BuildProduct<V> for ProductBuilder<D, C, T>
where
    D: DataSrc<DayCountSymbol, Output = DayCount>,
    C: DataSrc<CalendarSymbol, Output = Calendar>,
    T: DataSrc<str, Output = DateToDateTime>,
    V: Real,
{
    type Variables = VariableTypesExpanded<V>;

    #[inline]
    fn initialize(&mut self) {
        self.conv.lock().unwrap().clear();
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
        _: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        let values = cmp.values.iter();
        let values = values.map(|v| _unwrap_float(v, consts, "constant_float"));
        let values = values
            .collect::<anyhow::Result<Vec<_>>>()?
            .require_min_size()?;
        Ok(Arc::new(Process::ConstantFloat(ConstantFloat { values })))
    }

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicFloat<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        let mut series = Vec::default();
        for ser in cmp.series.iter() {
            let ts = ser.iter().map(|(dt, v)| {
                let dt = _unwrap_dt(dt, consts, "deterministic_float", &self.time_cut)?;
                let v = _unwrap_float(v, consts, "deterministic_float")?;
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
        _: &HashMap<String, Constant>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        let refs = cmp.refs.iter();
        let refs = refs.map(|m| {
            mkts.get(m)
                .ok_or_else(
                    || anyhow!("market `{m}` is not found which is required by market_ref",),
                )
                .cloned()
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
        _: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        Ok(Arc::new(CashflowWithFixing::FixedCoupon(FixedCoupon {
            base: _unwrap_cpnbase(
                &cmp.base,
                consts,
                "fixed_coupon",
                &self.time_cut,
                &self.daycnt_src,
            )?,
            rate: _unwrap_float(&cmp.rate, consts, "fixed_coupon")?,
            accrual: _unwrap_dcnt(&cmp.accrual, consts, "fixed_coupon", &self.daycnt_src)?,
            rounding: cmp
                .rounding
                .as_ref()
                .map(|r| _unwrap_rounding(r, consts, "fixed_coupon", &self.rounding))
                .transpose()?,
        })))
    }

    fn parse_cf_overnight_index_coupon(
        &self,
        cmp: &OvernightIndexCoupon<VariableTypesForData<V>>,
        fixing: Option<&OvernightIndexFixing>,
        consts: &HashMap<String, Constant>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        Ok(Arc::new(CashflowWithFixing::OvernightIndexCoupon(
            OvernightIndexCoupon {
                base: _unwrap_cpnbase(
                    &cmp.base,
                    consts,
                    "overnight_index_coupon",
                    &self.time_cut,
                    &self.daycnt_src,
                )?,
                convention: _unwrap_inarrears(
                    &cmp.convention,
                    consts,
                    "overnight_index_coupon",
                    &self.daycnt_src,
                    &self.cal_src,
                    &self.conv,
                )?,
                reference_rate: procs
                    .get(&cmp.reference_rate)
                    .ok_or_else(|| {
                        anyhow!(
                            "process `{}` is not found which is required by overnight_index_coupon",
                            cmp.reference_rate
                        )
                    })?
                    .clone(),
                gearing: cmp
                    .gearing
                    .as_ref()
                    .map(|v| _unwrap_float(v, consts, "overnight_index_coupon"))
                    .transpose()?,
                spread: cmp
                    .spread
                    .as_ref()
                    .map(|v| _unwrap_float(v, consts, "overnight_index_coupon"))
                    .transpose()?,
                rounding: cmp
                    .rounding
                    .as_ref()
                    .map(|r| _unwrap_rounding(r, consts, "overnight_index_coupon", &self.rounding))
                    .transpose()?,
            },
            fixing.cloned(),
        )))
    }

    // leg
    fn parse_leg_straight(
        &self,
        leg: &StraightLeg<VariableTypesForData<V>>,
        _: &HashMap<String, Constant>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
        _: &HashMap<String, <Self::Variables as VariableTypes>::LegRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::LegRef> {
        let cashflows = leg.cashflows.iter();
        let cashflows = cashflows.map(|cf| {
            cfs.get(cf)
                .ok_or_else(|| {
                    anyhow!("cashflow `{cf}` is not found which is required by straight_leg",)
                })
                .cloned()
        });
        let cashflows = cashflows.collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Arc::new(Leg::Straight(StraightLeg { cashflows })))
    }
}

fn _unwrap_float<V: Real>(
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
            Constant::Number(v) => Ok(*v),
            _ => bail!("constant `{id}` is not a number."),
        })
        .map(V::nearest_base_float_of)
        .map(Into::into)
}

fn _unwrap_dt<T>(
    v: &ValueOrId<DateWithTag>,
    consts: &HashMap<String, Constant>,
    required_by: &str,
    src: &T,
) -> anyhow::Result<DateTime>
where
    T: DataSrc<str, Output = DateToDateTime>,
{
    let id = match v {
        ValueOrId::Value(v) => {
            return v
                .to_datetime(src)
                .context("Converting ProductData to Product")
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
        .and_then(|dt| dt.to_datetime(src))
}

fn _unwrap_dcnt<D>(
    v: &ValueOrId<DayCountSymbol>,
    consts: &HashMap<String, Constant>,
    required_by: &str,
    src: &D,
) -> anyhow::Result<DayCount>
where
    D: DataSrc<DayCountSymbol, Output = DayCount>,
{
    let id = match v {
        ValueOrId::Value(v) => return src.get(v).context("Converting ProductData to Product"),
        ValueOrId::Id(id) => id,
    };
    consts
        .get(id)
        .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
        .and_then(|c| match c {
            Constant::Object(v) => DayCountSymbol::deserialize(v).map_err(Into::into),
            _ => bail!("constant `{id}` is not an object, which is expected a `DayCountSymbol`."),
        })
        .and_then(|s| src.get(&s).context("Converting ProductData to Product"))
}

fn _unwrap_rounding(
    v: &ValueOrId<Rounding>,
    consts: &HashMap<String, Constant>,
    required_by: &str,
    cache: &Mutex<HashMap<String, Rounding>>,
) -> anyhow::Result<Rounding> {
    let id = match v {
        ValueOrId::Value(v) => return Ok(*v),
        ValueOrId::Id(id) => id,
    };
    let mut cache = cache.lock().unwrap();
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

fn _unwrap_cpnbase<V, T, D>(
    v: &CouponBase<VariableTypesForData<V>>,
    consts: &HashMap<String, Constant>,
    required_by: &str,
    tcut_src: &T,
    dcnt_src: &D,
) -> anyhow::Result<CouponBase<VariableTypesExpanded<V>>>
where
    V: Real,
    T: DataSrc<str, Output = DateToDateTime>,
    D: DataSrc<DayCountSymbol, Output = DayCount>,
{
    let base = CouponBase {
        notional: Money {
            amount: _unwrap_float(&v.notional.amount, consts, required_by)?,
            ccy: v.notional.ccy,
        },
        period_start: _unwrap_dt(&v.period_start, consts, required_by, tcut_src)?,
        period_end: _unwrap_dt(&v.period_end, consts, required_by, tcut_src)?,
        entitle: _unwrap_dt(&v.entitle, consts, required_by, tcut_src)?,
        payment: _unwrap_dt(&v.payment, consts, required_by, tcut_src)?,
        daycount: _unwrap_dcnt(&v.daycount, consts, required_by, dcnt_src)?,
    };
    Ok(base)
}

fn _unwrap_inarrears<T, D>(
    v: &ValueOrId<InArrears<DayCountSymbol, CalendarSymbol>>,
    consts: &HashMap<String, Constant>,
    required_by: &str,
    dcnt_src: &D,
    cal_src: &T,
    cache: &Mutex<HashMap<String, Arc<InArrears<DayCount, Calendar>>>>,
) -> anyhow::Result<Arc<InArrears<DayCount, Calendar>>>
where
    T: DataSrc<CalendarSymbol, Output = Calendar>,
    D: DataSrc<DayCountSymbol, Output = DayCount>,
{
    let parse = |v: &InArrears<DayCountSymbol, CalendarSymbol>| {
        use InArrears::*;
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
    let mut cache = cache.lock().unwrap();
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
