use std::{
    collections::HashMap,
    hash::Hash,
    str::FromStr,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Context};
use qrs_chrono::{Calendar, CalendarSymbol, DateTime, DateToDateTime, DateWithTag};
use qrs_collections::RequireMinSize;
use qrs_datasrc::{DataSrc, DebugTree};
use qrs_math::{num::Real, rounding::Rounding};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    daycount::{DayCount, DayCountSymbol},
    product::{
        core::{Collateral, InArrears, SpreadExclusiveCompounding, StraightCompounding},
        general::{
            cashflow::{
                Cashflow, CashflowFixing, CashflowWithFixing, CouponBase, FixedCoupon,
                OvernightIndexCoupon, OvernightIndexFixing,
            },
            constant::Constant,
            core::{ComponentCategory, ComponentKey, ValueOrId, VariableTypes, WithId},
            leg::{Leg, StraightLeg},
            market::{Market, OvernightRate},
            process::{ConstantNumber, DeterministicNumber, MarketRef, Process, Ratio, ValueType},
        },
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
        for ComponentKey { cat, id: name } in dep.topological_sorted().iter().rev() {
            match cat {
                ComponentCategory::Constant => {}
                ComponentCategory::Market => {
                    if let Some(cmp) = product.mkts.remove(name.as_ref()) {
                        let mkt = self.convert_mkt(cmp)?;
                        mkts.insert(name.clone().0, mkt);
                    }
                }
                ComponentCategory::Process => {
                    if let Some(cmp) = product.procs.remove(name.as_ref()) {
                        let proc = self.convert_proc(cmp, &mkts)?;
                        procs.insert(name.clone().0, proc);
                    }
                }
                ComponentCategory::Cashflow => {
                    if let Some(cmp) = product.cfs.remove(name.as_ref()) {
                        let cf = self.convert_cf(cmp, &mkts, &procs)?;
                        cfs.insert(name.clone().0, cf);
                    }
                }
                ComponentCategory::Leg => {
                    if let Some(cmp) = product.legs.remove(name.as_ref()) {
                        let leg = self.convert_leg(cmp, &procs, &cfs)?;
                        legs.insert(name.clone().0, leg);
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
pub trait BuildProduct<V = f64>: Sized {
    type Variables: VariableTypes;

    fn initialize(&self) {}
    fn post_validation(&self, result: &Product<Self::Variables>) -> anyhow::Result<()> {
        let _ = result;
        Ok(())
    }

    // market
    fn parse_mkt_overnight_rate(
        &self,
        cmp: &OvernightRate,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::MarketRef>;

    // process
    fn parse_proc_constant_float(
        &self,
        cmp: &ConstantNumber<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicNumber<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_market_ref(
        &self,
        cmp: &MarketRef<VariableTypesForData<V>>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    fn parse_proc_ratio(
        &self,
        cmp: &Ratio<VariableTypesForData<V>>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef>;

    // cashflow
    fn parse_cf_fixed_coupon(
        &self,
        cmp: &FixedCoupon<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    fn parse_cf_overnight_index_coupon(
        &self,
        cmp: &OvernightIndexCoupon<VariableTypesForData<V>>,
        fixing: Option<&OvernightIndexFixing>,
        consts: &HashMap<String, Constant<V>>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef>;

    // leg
    fn parse_leg_straight(
        &self,
        leg: &StraightLeg<VariableTypesForData<V>>,
        cfs: &HashMap<String, <Self::Variables as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::LegRef>;

    // build
    fn build(&self, data: &ProductData<V>) -> anyhow::Result<Product<Self::Variables>> {
        self.initialize();
        let dep = data.contract._dependency()?;
        let mut mkts = HashMap::new();
        let mut procs = HashMap::new();
        let mut cfs = HashMap::new();
        let mut legs = HashMap::new();
        for ComponentKey { cat, id: name } in dep.topological_sorted().iter().rev() {
            match cat {
                ComponentCategory::Constant => {}
                ComponentCategory::Market => {
                    let cmp = data.contract.markets.get(name.as_ref()).unwrap();
                    let mkt = match cmp {
                        Market::OvernightRate(cmp) => self.parse_mkt_overnight_rate(cmp)?,
                    };
                    mkts.insert(name.clone().0, mkt);
                }
                ComponentCategory::Process => {
                    let cmp = data.contract.processes.get(name.as_ref()).unwrap();
                    let proc = match cmp {
                        Process::ConstantNumber(cmp) => {
                            self.parse_proc_constant_float(cmp, &data.contract.constants)?
                        }
                        Process::DeterministicNumber(cmp) => {
                            self.parse_proc_deternministic_float(cmp, &data.contract.constants)?
                        }
                        Process::MarketRef(cmp) => self.parse_proc_market_ref(cmp, &mkts)?,
                        Process::Ratio(cmp) => self.parse_proc_ratio(cmp, &procs)?,
                    };
                    procs.insert(name.clone().0, proc);
                }
                ComponentCategory::Cashflow => {
                    let cmp = data.contract.cashflows.get(name.as_ref()).unwrap();
                    let fixing = data.fixing.cashflows.get(name.as_ref());
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
                    cfs.insert(name.clone().0, cf);
                }
                ComponentCategory::Leg => {
                    let cmp = data.contract.legs.get(name.as_ref()).unwrap();
                    let leg = match cmp {
                        Leg::Straight(cmp) => self.parse_leg_straight(cmp, &cfs)?,
                    };
                    legs.insert(name.clone().0, leg);
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
    type Money = Money<V>;
    type Rounding = Rounding;

    type MarketRef = Arc<Market>;
    type ProcessRef = Arc<Process<Self>>;
    type CashflowRef = Arc<CashflowWithFixing<Self>>;
    type LegRef = Arc<Leg<Self>>;

    type InArrearsConvention = Arc<InArrears<DayCount, Calendar>>;
}

// -----------------------------------------------------------------------------
//  DefaultProductBuilder
//
#[derive(Debug, DebugTree)]
#[debug_tree(desc = "default product builder")]
pub struct DefaultProductBuilder<DayCnt = (), Cal = (), TimeCut = ()> {
    #[debug_tree(subtree)]
    daycnt_src: DayCnt,
    #[debug_tree(subtree)]
    cal_src: Cal,
    #[debug_tree(subtree)]
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
        DayCnt: DataSrc<DayCountSymbol, Output = DayCount>,
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
    fn initialize(&self) {
        self.conv.lock().unwrap().clear();
        self.rounding.lock().unwrap().clear();
    }

    // market
    #[inline]
    fn parse_mkt_overnight_rate(
        &self,
        cmp: &OvernightRate,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::MarketRef> {
        Ok(Arc::new(Market::OvernightRate(cmp.clone())))
    }

    // process
    fn parse_proc_constant_float(
        &self,
        cmp: &ConstantNumber<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        const NAME: &str = "constant_float";
        let values = cmp.values.iter();
        let values = values.map(|v| self._unwrap_float(v, consts, NAME));
        let values = values
            .collect::<anyhow::Result<Vec<_>>>()?
            .require_min_size()?;
        Ok(Arc::new(Process::ConstantNumber(ConstantNumber { values })))
    }

    fn parse_proc_deternministic_float(
        &self,
        cmp: &DeterministicNumber<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
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
        Ok(Arc::new(Process::DeterministicNumber(
            DeterministicNumber { series },
        )))
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
            mkts.get(id.as_ref())
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

    fn parse_proc_ratio(
        &self,
        cmp: &Ratio<VariableTypesForData<V>>,
        procs: &HashMap<String, <Self::Variables as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::ProcessRef> {
        const NAME: &str = "ratio";
        let num = procs.get(cmp.numer.id.as_ref()).ok_or_else(|| {
            anyhow!(
                "process `{}` is not found which is required by {NAME} as numerator",
                cmp.numer.id
            )
        })?;
        let den = procs.get(cmp.denom.id.as_ref()).ok_or_else(|| {
            anyhow!(
                "process `{}` is not found which is required by {NAME} as denominator",
                cmp.denom.id
            )
        })?;
        match (num.value_type()?, den.value_type()?) {
            (ValueType::Number { dim: num }, ValueType::Number { dim: den }) => {
                if num != den {
                    bail!("numerator and denominator of {NAME} must have the same dimension");
                }
                if num == 0 {
                    bail!("numerator and denominator of {NAME} must have non-zero dimension");
                }
            }
            _ => bail!("numerator and denominator of {NAME} must return number"),
        }
        Ok(Arc::new(Process::Ratio(Ratio {
            numer: WithId {
                id: cmp.numer.id.clone(),
                value: num.clone(),
            },
            denom: WithId {
                id: cmp.denom.id.clone(),
                value: den.clone(),
            },
        })))
    }

    //
    fn parse_cf_fixed_coupon(
        &self,
        cmp: &FixedCoupon<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        const NAME: &str = "fixed_coupon";
        Ok(Arc::new(CashflowWithFixing::FixedCoupon(FixedCoupon {
            base: self._unwrap_cpnbase(&cmp.base, consts, NAME)?,
            rate: self._unwrap_float(&cmp.rate, consts, NAME)?,
            accrued_daycount: self._unwrap_dcnt(&cmp.accrued_daycount, consts, NAME)?,
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
        consts: &HashMap<String, Constant<V>>,
        mkts: &HashMap<String, <Self::Variables as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<Self::Variables as VariableTypes>::CashflowRef> {
        const NAME: &str = "overnight_index_coupon";
        let ref_rate = mkts.get(cmp.reference_rate.id.as_ref()).ok_or_else(|| {
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
            cfs.get(id.as_ref())
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
        consts: &HashMap<String, Constant<V>>,
        required_by: &str,
    ) -> anyhow::Result<V> {
        let id = match v {
            ValueOrId::Value(v) => return Ok(v.clone()),
            ValueOrId::Id(id) => id,
        };
        consts
            .get(id.as_ref())
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Number(v) => Ok(v.clone()),
                _ => bail!("constant `{id}` is not a number."),
            })
    }

    fn _unwrap_money<V: Real>(
        &self,
        v: &ValueOrId<Money<V>>,
        consts: &HashMap<String, Constant<V>>,
        required_by: &str,
    ) -> anyhow::Result<Money<V>> {
        let id = match v {
            ValueOrId::Value(v) => return Ok(v.clone()),
            ValueOrId::Id(id) => id,
        };
        consts
            .get(id.as_ref())
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => {
                    let money = Money::<f64>::deserialize(v)?;
                    Ok(Money {
                        amount: V::nearest_base_float_of(money.amount).into(),
                        ccy: money.ccy,
                    })
                }
                _ => bail!("constant `{id}` is not an object, which is expected a `Money`."),
            })
    }

    fn _unwrap_dt<V: Real>(
        &self,
        v: &ValueOrId<DateWithTag>,
        consts: &HashMap<String, Constant<V>>,
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
            .get(id.as_ref())
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

    fn _unwrap_dcnt<V: Real>(
        &self,
        v: &ValueOrId<DayCountSymbol>,
        consts: &HashMap<String, Constant<V>>,
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
            .get(id.as_ref())
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

    fn _unwrap_rounding<V: Real>(
        &self,
        v: &ValueOrId<Rounding>,
        consts: &HashMap<String, Constant<V>>,
        required_by: &str,
    ) -> anyhow::Result<Rounding> {
        let id = match v {
            ValueOrId::Value(v) => return Ok(*v),
            ValueOrId::Id(id) => id,
        };
        let mut cache = self.rounding.lock().unwrap();
        if let Some(res) = cache.get(id.as_ref()) {
            return Ok(*res);
        }
        let res = consts
            .get(id.as_ref())
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => Rounding::deserialize(v)
                    .map_err(Into::<anyhow::Error>::into)
                    .context("Converting ProductData to Product"),
                _ => bail!("constant `{id}` is not an object, which is expected a `Rounding`."),
            })?;
        cache.insert(id.clone().into(), res);
        Ok(res)
    }

    fn _unwrap_cpnbase<V: Real>(
        &self,
        v: &CouponBase<VariableTypesForData<V>>,
        consts: &HashMap<String, Constant<V>>,
        required_by: &str,
    ) -> anyhow::Result<CouponBase<DefaultVariableTypes<V>>> {
        let base = CouponBase {
            notional: self._unwrap_money(&v.notional, consts, required_by)?,
            period_start: self._unwrap_dt(&v.period_start, consts, required_by)?,
            period_end: self._unwrap_dt(&v.period_end, consts, required_by)?,
            entitle: self._unwrap_dt(&v.entitle, consts, required_by)?,
            payment: self._unwrap_dt(&v.payment, consts, required_by)?,
            daycount: self._unwrap_dcnt(&v.daycount, consts, required_by)?,
        };
        Ok(base)
    }

    fn _unwrap_inarrears<V: Real>(
        &self,
        v: &ValueOrId<InArrears<DayCountSymbol, CalendarSymbol>>,
        consts: &HashMap<String, Constant<V>>,
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
        if let Some(res) = cache.get(id.as_ref()) {
            return Ok(res.clone());
        }
        let res = consts
            .get(id.as_ref())
            .ok_or_else(|| anyhow!("constant `{id}` is required by {required_by} but not found."))
            .and_then(|c| match c {
                Constant::Object(v) => InArrears::<DayCountSymbol, CalendarSymbol>::deserialize(v)
                    .map_err(Into::<anyhow::Error>::into)
                    .context("Converting ProductData to Product"),
                _ => bail!("constant `{id}` is not an object, which is expected a `InArrears`."),
            })
            .and_then(|s| parse(&s).context("Converting ProductData to Product"))?;
        cache.insert(id.clone().into(), res.clone());
        Ok(res)
    }
}

impl<D, C, T, V> DataSrc<ProductData<V>> for DefaultProductBuilder<D, C, T>
where
    D: DataSrc<DayCountSymbol, Output = DayCount>,
    C: DataSrc<CalendarSymbol, Output = Calendar>,
    T: DataSrc<str, Output = DateToDateTime>,
    V: Real,
{
    type Output = Product<DefaultVariableTypes<V>>;

    #[inline]
    fn get(&self, data: &ProductData<V>) -> anyhow::Result<Product<DefaultVariableTypes<V>>> {
        self.build(data)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use mockall::mock;
    use qrs_datasrc::{DataSrc, DebugTree, TreeInfo};

    use crate::{daycount::DayCountSrc, market};

    use super::*;

    mock! {
        CalSrc {}

        impl DebugTree for CalSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> TreeInfo;
        }

        impl DataSrc<CalendarSymbol> for CalSrc {
            type Output = Calendar;

            fn get(&self, symbol: &CalendarSymbol) -> anyhow::Result<Calendar>;
        }
    }
    mock! {
        TimeCutSrc {}

        impl DebugTree for TimeCutSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> TreeInfo;
        }

        impl DataSrc<str> for TimeCutSrc {
            type Output = DateToDateTime;

            fn get(&self, symbol: &str) -> anyhow::Result<DateToDateTime>;
        }
    }

    impl MockCalSrc {
        fn set_count(&mut self, n: usize) {
            self.expect_get()
                .times(n)
                .returning(|_| Ok(Calendar::default()));
        }
    }
    impl MockTimeCutSrc {
        fn set_count(&mut self, n: usize) {
            self.expect_get()
                .times(n)
                .returning(|_| "15:30T+09:00".parse().map_err(Into::into));
        }
    }
    struct MockSrc {
        cal: Arc<Mutex<MockCalSrc>>,
        tct: Arc<Mutex<MockTimeCutSrc>>,
        dct: Arc<Mutex<DayCountSrc<MockCalSrc>>>,
    }
    impl MockSrc {
        fn set_cal_count(&mut self, n: usize) -> &mut Self {
            self.cal.lock().unwrap().set_count(n);
            self
        }
        fn set_tct_count(&mut self, n: usize) -> &mut Self {
            self.tct.lock().unwrap().set_count(n);
            self
        }
        fn set_dct_count(&mut self, n: usize) -> &mut Self {
            self.dct.lock().unwrap().inner_mut().set_count(n);
            self
        }
        fn checkpoint(&mut self) {
            self.cal.lock().unwrap().checkpoint();
            self.tct.lock().unwrap().checkpoint();
            self.dct.lock().unwrap().inner_mut().checkpoint();
        }
    }

    #[allow(clippy::type_complexity)]
    fn fixture() -> (
        MockSrc,
        DefaultProductBuilder<
            Arc<Mutex<DayCountSrc<MockCalSrc>>>,
            Arc<Mutex<MockCalSrc>>,
            Arc<Mutex<MockTimeCutSrc>>,
        >,
    ) {
        let cal = Arc::new(Mutex::new(MockCalSrc::new()));
        let dct = Arc::new(Mutex::new(DayCountSrc::new(MockCalSrc::new())));
        let tct = Arc::new(Mutex::new(MockTimeCutSrc::new()));
        let builder = DefaultProductBuilder::new()
            .with_daycount_src(dct.clone())
            .with_calendar_src(cal.clone())
            .with_time_cut(tct.clone());
        (MockSrc { dct, cal, tct }, builder)
    }

    #[test]
    fn test_parse_mkt_overnight_rate() {
        let (mut mock, builder) = fixture();
        mock.set_cal_count(0).set_tct_count(0).set_dct_count(0);
        let mkt = OvernightRate {
            reference: market::ir::OvernightRate::TONA,
        };

        let res = BuildProduct::<f64>::parse_mkt_overnight_rate(&builder, &mkt).unwrap();

        assert_eq!(res, Arc::new(Market::OvernightRate(mkt)));
        mock.checkpoint();
    }

    #[test]
    fn test_parse_proc_constant_float() {
        let (mut mock, builder) = fixture();
        mock.set_cal_count(0).set_tct_count(0).set_dct_count(0);
        let cmp = ConstantNumber {
            values: vec![
                ValueOrId::Id("c0".into()),
                ValueOrId::Value(1.0),
                ValueOrId::Id("c1".into()),
            ]
            .require_min_size()
            .unwrap(),
        };
        let consts = hashmap! {
            "c0".into() => Constant::Number(42.0),
            "c1".into() => Constant::Number(24.0),
        };

        let res = BuildProduct::<f64>::parse_proc_constant_float(&builder, &cmp, &consts).unwrap();

        assert!(matches!(res.as_ref(), Process::ConstantNumber(_)));
        let Process::ConstantNumber(res) = res.as_ref() else {
            panic!()
        };
        assert_eq!(
            res,
            &ConstantNumber {
                values: vec![42.0f64, 1.0f64, 24.0f64].require_min_size().unwrap()
            }
        );
        mock.checkpoint();
    }
}
