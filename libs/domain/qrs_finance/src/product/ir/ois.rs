use std::{collections::HashMap, sync::Arc};

use anyhow::bail;
use qrs_chrono::{Calendar, DateTime};
use qrs_math::{num::Real, rounding::Rounding};
use schemars::JsonSchema;

use crate::{
    daycount::DayCount,
    product::{
        core::InArrears,
        general::{
            cashflow::{
                CashflowWithFixing, FixedCoupon, OvernightIndexCoupon, OvernightIndexFixing,
            },
            core::{VariableTypes, WithId},
            leg::{Leg, StraightLeg},
            market::{Market, OvernightRate},
            ConvertProduct, DefaultVariableTypes, Product,
        },
    },
    Money,
};

// -----------------------------------------------------------------------------
// OisVariableTypes
//
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct OisVariableTypes<V = f64>(std::marker::PhantomData<V>);

//
// methods
//
impl<V> VariableTypes for OisVariableTypes<V> {
    type Boolean = bool;
    type Integer = i64;
    type Number = V;

    type Calendar = Calendar;
    type DateTime = DateTime;
    type DayCount = DayCount;
    type Rounding = Rounding;
    type Money = Money<V>;

    type MarketRef = Arc<OvernightRate>;
    type ProcessRef = ();
    type CashflowRef = Arc<OisCashflow<V>>;
    type LegRef = Arc<StraightLeg<Self>>;

    type InArrearsConvention = Arc<InArrears<DayCount, Calendar>>;
}

// -----------------------------------------------------------------------------
// OisCashflow
//
#[derive(Debug, Clone, PartialEq)]
pub enum OisCashflow<V> {
    Fixed(FixedCoupon<OisVariableTypes<V>>),
    OvernightIndex(
        OvernightIndexCoupon<OisVariableTypes<V>>,
        OvernightIndexFixing,
    ),
}

// -----------------------------------------------------------------------------
// OisConverter
//
#[derive(Debug, Clone, PartialEq)]
pub struct OisConverter {}

//
// methods
//
impl<V: Real> ConvertProduct<DefaultVariableTypes<V>, OisVariableTypes<V>> for OisConverter {
    fn initialize(&mut self) {}
    fn post_validation(&self, _: &Product<OisVariableTypes<V>>) -> anyhow::Result<()> {
        Ok(())
    }

    fn convert_mkt(
        &self,
        cmp: <DefaultVariableTypes<V> as VariableTypes>::MarketRef,
    ) -> anyhow::Result<<OisVariableTypes<V> as VariableTypes>::MarketRef> {
        let cmp = match Arc::try_unwrap(cmp) {
            Ok(cmp) => cmp,
            Err(cmp) => cmp.as_ref().clone(),
        };
        match cmp {
            Market::OvernightRate(cmp) => Ok(Arc::new(cmp)),
        }
    }

    fn convert_proc(
        &self,
        _: <DefaultVariableTypes<V> as VariableTypes>::ProcessRef,
        _: &HashMap<String, <OisVariableTypes<V> as VariableTypes>::MarketRef>,
    ) -> anyhow::Result<<OisVariableTypes<V> as VariableTypes>::ProcessRef> {
        bail!("OIS does not support process component")
    }

    fn convert_cf(
        &self,
        cmp: <DefaultVariableTypes<V> as VariableTypes>::CashflowRef,
        mkts: &HashMap<String, <OisVariableTypes<V> as VariableTypes>::MarketRef>,
        _: &HashMap<String, <OisVariableTypes<V> as VariableTypes>::ProcessRef>,
    ) -> anyhow::Result<<OisVariableTypes<V> as VariableTypes>::CashflowRef> {
        let cmp = match Arc::try_unwrap(cmp) {
            Ok(cmp) => cmp,
            Err(cmp) => cmp.as_ref().clone(),
        };
        let cf = match cmp {
            CashflowWithFixing::FixedCoupon(cmp) => {
                OisCashflow::Fixed(cmp.change_variable_types_to())
            }
            CashflowWithFixing::OvernightIndexCoupon(cmp, fixing) => {
                let cf = OvernightIndexCoupon {
                    base: cmp.base.change_variable_types_to(),
                    convention: cmp.convention,
                    reference_rate: WithId {
                        id: cmp.reference_rate.id.clone(),
                        value: mkts
                            .get(cmp.reference_rate.id.as_ref())
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Market component `{}` not found",
                                    cmp.reference_rate.id
                                )
                            })?
                            .clone(),
                    },
                    spread: cmp.spread.map(Into::into),
                    gearing: cmp.gearing.map(Into::into),
                    rounding: cmp.rounding.map(Into::into),
                };
                OisCashflow::OvernightIndex(cf, fixing.unwrap())
            }
        };
        Ok(Arc::new(cf))
    }

    fn convert_leg(
        &self,
        cmp: <DefaultVariableTypes<V> as VariableTypes>::LegRef,
        _: &HashMap<String, <OisVariableTypes<V> as VariableTypes>::ProcessRef>,
        cfs: &HashMap<String, <OisVariableTypes<V> as VariableTypes>::CashflowRef>,
    ) -> anyhow::Result<<OisVariableTypes<V> as VariableTypes>::LegRef> {
        let Leg::Straight(cmp) = cmp.as_ref();
        let cashflows = cmp
            .cashflows
            .iter()
            .map(|cf| {
                cfs.get(cf.id.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("Cashflow component `{}` not found", cf.id))
                    .map(|value| WithId {
                        id: cf.id.clone(),
                        value: value.clone(),
                    })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Arc::new(StraightLeg { cashflows }))
    }
}
