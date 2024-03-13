use std::{collections::HashMap, fmt::Debug, sync::Arc};

use anyhow::bail;
use qrs_chrono::{Calendar, CalendarSymbol, DateTime, DateWithTag, TimeCut, Tz};
use qrs_collections::RequireMinSize;
use qrs_datasrc::{DataSrc, DebugTree};
use serde::Deserialize;

use crate::{
    core::{
        daycount::{DayCount, DayCountSymbol},
        Money,
    },
    products::{
        core::{Collateral, CompoundingConvention},
        general::{
            cashflow::{CouponBase, FixedCoupon, OvernightIndexCoupon},
            constant::Constant,
            core::{ComponentCategory, ComponentKey, VariableTypes},
            leg::StraightLeg,
            process::{ConstantFloat, DeterministicFloat, MarketRef},
        },
    },
};

use super::{
    super::{cashflow::Cashflow, leg::Leg, market::Market, process::Process},
    data::{ComponentDependency, ValueOrId},
    ProductData, VariableTypesForData,
};

// -----------------------------------------------------------------------------
// VariableTypesForEval
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(schemars::JsonSchema))]
pub struct VariableTypesForEval<V>(std::marker::PhantomData<V>);

impl<V> VariableTypes for VariableTypesForEval<V> {
    type Number = V;
    type Integer = i64;
    type Boolean = bool;

    type DateTime = DateTime;
    type DayCount = DayCount;
    type Calendar = Calendar;

    type CashflowRef = Arc<Cashflow<VariableTypesForEval<V>>>;
    type LegRef = Arc<Leg<VariableTypesForEval<V>>>;
    type MarketRef = Arc<Market>;
    type ProcessRef = Arc<Process<VariableTypesForEval<V>>>;

    type CompoundingConvention = Arc<CompoundingConvention<DayCount, Calendar>>;
}

// -----------------------------------------------------------------------------
// GeneralProduct
//
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralProduct<V = f64> {
    dep: ComponentDependency,
    collateral: Collateral,
    markets: HashMap<String, Arc<Market>>,
    processes: HashMap<String, Arc<Process<VariableTypesForEval<V>>>>,
    cashflows: HashMap<String, Arc<Cashflow<VariableTypesForEval<V>>>>,
    legs: HashMap<String, Arc<Leg<VariableTypesForEval<V>>>>,
}

//
// methods
//
impl GeneralProduct {
    /// Create a new [GeneralProductBuilder] which converts from [ProductData] to [GeneralProduct]
    #[inline]
    pub fn parser() -> GeneralProductBuilder {
        GeneralProductBuilder::new()
    }
}

impl<V> GeneralProduct<V> {
    /// Get the dependency information of the product components
    #[inline]
    pub fn dependency(&self) -> &ComponentDependency {
        &self.dep
    }

    /// Get the collateral of the product
    #[inline]
    pub fn collateral(&self) -> &Collateral {
        &self.collateral
    }

    /// Get the market components
    #[inline]
    pub fn markets(&self) -> &HashMap<String, Arc<Market>> {
        &self.markets
    }

    /// Get the process components
    #[inline]
    pub fn processes(&self) -> &HashMap<String, Arc<Process<VariableTypesForEval<V>>>> {
        &self.processes
    }

    /// Get the cashflow components
    #[inline]
    pub fn cashflows(&self) -> &HashMap<String, Arc<Cashflow<VariableTypesForEval<V>>>> {
        &self.cashflows
    }

    /// Get the leg components
    #[inline]
    pub fn legs(&self) -> &HashMap<String, Arc<Leg<VariableTypesForEval<V>>>> {
        &self.legs
    }
}

// -------------------------------------------------------------------------
// GeneralProductBuilder
//
#[derive(Debug, Clone, PartialEq, DebugTree)]
#[debug_tree(desc = "converter from SerializableGeneralProduct to GeneralProduct")]
pub struct GeneralProductBuilder<Cal = (), DayCnt = (), TimeCut = ()> {
    #[debug_tree(subtree)]
    cal: Cal,
    #[debug_tree(subtree)]
    daycnt: DayCnt,
    #[debug_tree(subtree)]
    timecut: TimeCut,
}

//
// construction
//
impl Default for GeneralProductBuilder<(), (), ()> {
    #[inline]
    fn default() -> Self {
        Self {
            cal: (),
            daycnt: (),
            timecut: (),
        }
    }
}

impl GeneralProductBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Cal, DayCnt> GeneralProductBuilder<Cal, DayCnt, ()> {
    /// Set the timecut source
    #[inline]
    pub fn with_timecut_src<D2Dt>(self, src: D2Dt) -> GeneralProductBuilder<Cal, DayCnt, D2Dt>
    where
        D2Dt: DataSrc<str>,
        D2Dt::Output: TimeCut<Tz = Tz>,
        anyhow::Error: From<<D2Dt::Output as TimeCut>::Err>,
    {
        GeneralProductBuilder {
            cal: self.cal,
            daycnt: self.daycnt,
            timecut: src,
        }
    }
}

impl<Cal, D2Dt> GeneralProductBuilder<Cal, (), D2Dt> {
    /// Set the daycounter source
    #[inline]
    pub fn with_daycnt_src<DayCnt>(self, src: DayCnt) -> GeneralProductBuilder<Cal, DayCnt, D2Dt>
    where
        DayCnt: DataSrc<DayCountSymbol, Output = DayCount>,
    {
        GeneralProductBuilder {
            cal: self.cal,
            daycnt: src,
            timecut: self.timecut,
        }
    }
}

impl<DayCnt, D2Dt> GeneralProductBuilder<(), DayCnt, D2Dt> {
    /// Set the calendar source
    #[inline]
    pub fn with_cal_src<Cal>(self, src: Cal) -> GeneralProductBuilder<Cal, DayCnt, D2Dt>
    where
        Cal: DataSrc<CalendarSymbol, Output = Calendar>,
    {
        GeneralProductBuilder {
            cal: src,
            daycnt: self.daycnt,
            timecut: self.timecut,
        }
    }
}

impl<Cal, DayCnt, D2Dt> GeneralProductBuilder<Cal, DayCnt, D2Dt>
where
    Cal: DataSrc<CalendarSymbol, Output = Calendar>,
    DayCnt: DataSrc<DayCountSymbol, Output = DayCount>,
    D2Dt: DataSrc<str>,
    D2Dt::Output: TimeCut<Tz = Tz>,
    anyhow::Error: From<<D2Dt::Output as TimeCut>::Err>,
{
    #[inline]
    pub fn build<'a, V>(&self, data: &'a ProductData<V>) -> anyhow::Result<GeneralProduct<V>>
    where
        V: Clone + From<f64> + Deserialize<'a>,
    {
        let dep = data.dependency()?;
        let mut mkts = HashMap::new();
        let mut proc = HashMap::new();
        let mut cfs = HashMap::new();
        let mut legs = HashMap::new();

        let mut consts = _ConstantStorage::default();

        for ComponentKey { ref cat, ref name } in dep.ordered_nodes().iter().rev() {
            use ComponentCategory::*;
            match cat {
                Constant(_) => {}
                Market => {
                    let Some(comp) = data.markets.get_key_value(name) else {
                        bail!("Market not found: {name}");
                    };
                    let parsed = self._parse_market(comp.1)?;
                    mkts.insert(comp.0.clone(), Arc::new(parsed));
                }
                Process(_) => {
                    let Some(comp) = data.processes.get_key_value(name) else {
                        bail!("Process not found: {name}");
                    };
                    let parsed = self._parse_process(comp.1, data, &mkts)?;
                    proc.insert(comp.0.clone(), Arc::new(parsed));
                }
                Cashflow => {
                    let Some(comp) = data.cashflows.get_key_value(name) else {
                        bail!("Cashflow not found: {name}");
                    };
                    let parsed = self._parse_cashflow(comp.1, data, &proc, &mut consts)?;
                    cfs.insert(comp.0.clone(), Arc::new(parsed));
                }
                Leg => {
                    let Some(comp) = data.legs.get_key_value(name) else {
                        bail!("Leg not found: {name}");
                    };
                    let parsed = self._parse_leg(comp.1, &cfs)?;
                    legs.insert(comp.0.clone(), Arc::new(parsed));
                }
            }
        }
        Ok(GeneralProduct {
            dep,
            collateral: data.collateral.clone(),
            markets: mkts,
            processes: proc,
            cashflows: cfs,
            legs,
        })
    }
}

// private impls
impl<Cal, DayCnt, D2Dt> GeneralProductBuilder<Cal, DayCnt, D2Dt>
where
    Cal: DataSrc<CalendarSymbol, Output = Calendar>,
    DayCnt: DataSrc<DayCountSymbol, Output = DayCount>,
    D2Dt: DataSrc<str>,
    D2Dt::Output: TimeCut<Tz = Tz>,
    anyhow::Error: From<<D2Dt::Output as TimeCut>::Err>,
{
    fn _parse_market(&self, comp: &Market) -> anyhow::Result<Market> {
        Ok(comp.clone())
    }

    fn _parse_process<V>(
        &self,
        comp: &Process<VariableTypesForData<V>>,
        data: &ProductData<V>,
        market: &HashMap<String, Arc<Market>>,
    ) -> anyhow::Result<Process<VariableTypesForEval<V>>>
    where
        V: Clone + From<f64>,
    {
        let res = match comp {
            Process::ConstantFloat(c) => {
                let values = c.values.iter().map(|v| _unwrap_number(v, &data.constants));
                Process::ConstantFloat(ConstantFloat {
                    values: values.collect::<Result<Vec<_>, _>>()?.require_min_size()?,
                })
            }
            Process::DeterministicFloat(c) => {
                let unwrap_map = |m: &HashMap<DateWithTag, ValueOrId<V>>| {
                    m.iter()
                        .map(|(k, v)| {
                            anyhow::Ok((
                                k.to_datetime(&self.timecut)?,
                                _unwrap_number(v, &data.constants)?,
                            ))
                        })
                        .collect::<Result<HashMap<_, _>, _>>()
                };
                let ser = c.series.iter();
                Process::DeterministicFloat(DeterministicFloat {
                    series: ser
                        .map(|m| {
                            unwrap_map(m).and_then(|m| m.require_min_size().map_err(Into::into))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .require_min_size()?,
                })
            }
            Process::Market(c) => {
                let id2mkt = |id: &String| {
                    market
                        .get(id)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Market not found: {}", id))
                };
                let refs = c.refs.iter();
                Process::Market(MarketRef {
                    refs: refs
                        .map(id2mkt)
                        .collect::<Result<Vec<_>, _>>()?
                        .require_min_size()?,
                })
            }
        };
        Ok(res)
    }

    fn _parse_cashflow<'a, V>(
        &self,
        comp: &Cashflow<VariableTypesForData<V>>,
        data: &'a ProductData<V>,
        process: &HashMap<String, Arc<Process<VariableTypesForEval<V>>>>,
        consts: &mut _ConstantStorage,
    ) -> anyhow::Result<Cashflow<VariableTypesForEval<V>>>
    where
        V: Clone + From<f64> + Deserialize<'a>,
    {
        let res = match comp {
            Cashflow::FixedCoupon(c) => Cashflow::FixedCoupon(FixedCoupon {
                base: self._parse_coupon_base(&c.base, data)?,
                rate: _unwrap_number(&c.rate, &data.constants)?,
            }),
            Cashflow::OvernightIndexCoupon(c) => {
                Cashflow::OvernightIndexCoupon(OvernightIndexCoupon {
                    base: self._parse_coupon_base(&c.base, data)?,
                    reference_rate: process.get(c.reference_rate.as_str()).cloned().ok_or_else(
                        || anyhow::anyhow!("Process not found: {}", c.reference_rate),
                    )?,
                    convention: {
                        if let Some(cc) = consts.compounding.get(&c.convention) {
                            cc.clone()
                        } else {
                            let cc = match data.constants.get(&c.convention) {
                                None => bail!("Constant not found: {}", c.convention),
                                Some(Constant::Object(v)) => v,
                                _ => bail!("Constant is not an object: {}", c.convention),
                            };
                            let cc = CompoundingConvention::<DayCountSymbol, CalendarSymbol>::deserialize(cc)?;
                            let cc = Arc::new(CompoundingConvention {
                                daycount: self.daycnt.get(&cc.daycount)?,
                                calendar: self.cal.get(&cc.calendar)?,
                                floor_target: cc.floor_target,
                                lockout: cc.lockout,
                                lookback: cc.lookback,
                            });
                            consts.compounding.insert(c.convention.clone(), cc.clone());
                            cc
                        }
                    },
                    gearing: c
                        .gearing
                        .as_ref()
                        .map(|v| _unwrap_number(v, &data.constants))
                        .transpose()?,
                    spread: c
                        .spread
                        .as_ref()
                        .map(|v| _unwrap_number(v, &data.constants))
                        .transpose()?,
                })
            }
        };
        Ok(res)
    }

    fn _parse_leg<V>(
        &self,
        comp: &Leg<VariableTypesForData<V>>,
        cfs: &HashMap<String, Arc<Cashflow<VariableTypesForEval<V>>>>,
    ) -> anyhow::Result<Leg<VariableTypesForEval<V>>>
    where
        V: Clone + From<f64>,
    {
        let res = match comp {
            Leg::Straight(c) => Leg::Straight(StraightLeg {
                cashflows: c
                    .cashflows
                    .iter()
                    .map(|cf| {
                        cfs.get(cf)
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("Cashflow not found: {}", cf))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        };
        Ok(res)
    }

    fn _parse_coupon_base<V>(
        &self,
        comp: &CouponBase<VariableTypesForData<V>>,
        data: &ProductData<V>,
    ) -> anyhow::Result<CouponBase<VariableTypesForEval<V>>>
    where
        V: Clone + From<f64>,
    {
        Ok(CouponBase {
            notional: Money {
                amount: _unwrap_number(&comp.notional.amount, &data.constants)?,
                ccy: comp.notional.ccy,
            },
            daycount: self.daycnt.get(&comp.daycount)?,
            entitle: comp.entitle.to_datetime(&self.timecut)?,
            payment: comp.payment.to_datetime(&self.timecut)?,
            period_start: comp.period_start.to_datetime(&self.timecut)?,
            period_end: comp.period_end.to_datetime(&self.timecut)?,
        })
    }
}

fn _unwrap_number<V>(v: &ValueOrId<V>, constants: &HashMap<String, Constant>) -> anyhow::Result<V>
where
    V: Clone + From<f64>,
{
    let res = match v {
        ValueOrId::Value(v) => v.clone(),
        ValueOrId::Id(id) => match constants.get(id) {
            None => bail!("Constant not found: {}", id),
            Some(Constant::Number(f)) => (*f).into(),
            _ => bail!("Constant is not a number: {}", id),
        },
    };
    Ok(res)
}

#[derive(Debug, Default)]
struct _ConstantStorage {
    compounding: HashMap<String, Arc<CompoundingConvention<DayCount, Calendar>>>,
}
