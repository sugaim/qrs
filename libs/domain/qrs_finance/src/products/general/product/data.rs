use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use qrs_chrono::{CalendarSymbol, DateWithTag};
use qrs_math::rounding::Rounding;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    daycount::DayCountSymbol,
    products::{
        core::{Collateral, InArrears},
        general::{
            cashflow::CashflowFixing,
            core::{Component, ComponentKey, ValueLess, ValueOrId, VariableTypes},
        },
    },
    Money,
};

use super::super::{
    cashflow::Cashflow, constant::Constant, leg::Leg, market::Market, process::Process,
};

// -----------------------------------------------------------------------------
// DependencyError
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub(crate) enum _DependencyError {
    #[error("{} is required by {} but not found", .required, .by)]
    MissingRequiredDependency {
        required: ComponentKey,
        by: ComponentKey,
    },
    #[error("circular dependency detected because all of the components are required by other components")]
    NoRootComponent,
    #[error("circular dependency detected at {}", .at)]
    Circular { at: ComponentKey },
}

// -----------------------------------------------------------------------------
// ComponentDependency
//
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub(crate) struct _ComponentDependency {
    edges: HashMap<ComponentKey, HashSet<ComponentKey>>,
    order: Vec<ComponentKey>,
}

//
// display, serde
//
impl _ComponentDependency {
    #[inline]
    pub fn topological_sorted(&self) -> &[ComponentKey] {
        &self.order
    }
}

// -----------------------------------------------------------------------------
// VariableTypesForData
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, schemars::JsonSchema)]
pub struct VariableTypesForData<V = f64>(std::marker::PhantomData<V>);

impl<V> VariableTypes for VariableTypesForData<V> {
    type Number = ValueOrId<V>;
    type Integer = ValueOrId<i64>;
    type Boolean = ValueOrId<bool>;

    type DateTime = ValueOrId<DateWithTag>;
    type DayCount = ValueOrId<DayCountSymbol>;
    type Calendar = ValueOrId<CalendarSymbol>;
    type Rounding = ValueOrId<Rounding>;
    type Money = ValueOrId<Money<V>>;

    type CashflowRef = ValueLess;
    type LegRef = ValueLess;
    type MarketRef = ValueLess;
    type ProcessRef = ValueLess;

    type InArrearsConvention = ValueOrId<InArrears<DayCountSymbol, CalendarSymbol>>;
}

// -----------------------------------------------------------------------------
// ContractData
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContractData<V = f64> {
    pub collateral: Collateral,

    #[serde(
        default = "HashMap::<String, Constant>::new",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub constants: HashMap<String, Constant>,

    #[serde(
        default = "HashMap::<String, Market>::new",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub markets: HashMap<String, Market>,

    #[serde(
        default = "HashMap::<String, Process<VariableTypesForData<V>>>::new",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub processes: HashMap<String, Process<VariableTypesForData<V>>>,

    #[serde(
        default = "HashMap::<String, Cashflow<VariableTypesForData<V>>>::new",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub cashflows: HashMap<String, Cashflow<VariableTypesForData<V>>>,

    #[serde(
        default = "HashMap::<String, Leg<VariableTypesForData<V>>>::new",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub legs: HashMap<String, Leg<VariableTypesForData<V>>>,
}

//
// methods
//
impl<V> ContractData<V> {
    pub(crate) fn _dependency(&self) -> Result<_ComponentDependency, _DependencyError> {
        // collect all the dependencies
        // key is dependent from, values are dependent to
        let edges = {
            let mut edges: HashMap<_, HashSet<_>> = HashMap::new();
            for (from, comp) in &self.constants {
                let key = ComponentKey {
                    cat: comp.category(),
                    name: from.clone(),
                };
                let node = edges.entry(key).or_default();
                node.extend(comp.depends_on().into_iter().map(|(to, cat)| ComponentKey {
                    cat,
                    name: to.to_string(),
                }));
            }
            for (from, comp) in &self.markets {
                let key = ComponentKey {
                    cat: comp.category(),
                    name: from.clone(),
                };
                let node = edges.entry(key).or_default();
                node.extend(comp.depends_on().into_iter().map(|(to, cat)| ComponentKey {
                    cat,
                    name: to.to_string(),
                }));
            }
            for (from, comp) in &self.processes {
                let key = ComponentKey {
                    cat: comp.category(),
                    name: from.clone(),
                };
                let node = edges.entry(key).or_default();
                node.extend(comp.depends_on().into_iter().map(|(to, cat)| ComponentKey {
                    cat,
                    name: to.to_string(),
                }));
            }
            for (from, comp) in &self.cashflows {
                let key = ComponentKey {
                    cat: comp.category(),
                    name: from.clone(),
                };
                let node = edges.entry(key).or_default();
                node.extend(comp.depends_on().into_iter().map(|(to, cat)| ComponentKey {
                    cat,
                    name: to.to_string(),
                }));
            }
            for (from, comp) in &self.legs {
                let key = ComponentKey {
                    cat: comp.category(),
                    name: from.clone(),
                };
                let node = edges.entry(key).or_default();
                node.extend(comp.depends_on().into_iter().map(|(to, cat)| ComponentKey {
                    cat,
                    name: to.to_string(),
                }));
            }
            edges
        };

        // check missing dependencies
        for (from, tos) in &edges {
            for to in tos {
                if !edges.contains_key(to) {
                    return Err(_DependencyError::MissingRequiredDependency {
                        required: to.clone(),
                        by: from.clone(),
                    });
                }
            }
        }

        // do topological sort
        // we consider A -> B if A depends on B
        // so in-degree is the number of components which requires the component.
        let mut num_required = HashMap::new();
        for depended in edges.values().flatten() {
            *num_required.entry(depended).or_insert(0) += 1;
        }

        let mut no_required = Vec::new();
        for from in edges.keys() {
            if !num_required.contains_key(from) {
                no_required.push(from.clone());
            }
        }
        if no_required.is_empty() {
            return Err(_DependencyError::NoRootComponent);
        }

        // less depended components come first and push to the sorted list.
        // so the first element is not depended by any other components.
        let mut sorted = Vec::new();
        while let Some(from) = no_required.pop() {
            sorted.push(from);
            for to in edges.get(sorted.last().unwrap()).into_iter().flatten() {
                if let Some(n) = num_required.get_mut(to) {
                    *n -= 1;
                    if *n == 0 {
                        no_required.push(to.clone());
                    }
                }
            }
        }

        if sorted.len() != edges.len() {
            let at = edges.into_keys().find(|k| !sorted.contains(k)).unwrap();
            return Err(_DependencyError::Circular { at });
        }

        Ok(_ComponentDependency {
            edges,
            order: sorted,
        })
    }
}

// -----------------------------------------------------------------------------
// FixingData
//
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FixingData {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub cashflows: HashMap<String, CashflowFixing>,
}

// -----------------------------------------------------------------------------
// ProductData
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProductData<V = f64> {
    pub contract: ContractData<V>,
    #[serde(default)]
    pub fixing: FixingData,
}
