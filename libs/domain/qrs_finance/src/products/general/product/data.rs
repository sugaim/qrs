use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use qrs_chrono::{CalendarSymbol, DateWithTag};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    core::daycount::DayCountSymbol,
    products::{
        core::Collateral,
        general::core::{Component, ComponentField, ComponentKey, VariableTypes},
    },
};

use super::super::{
    cashflow::Cashflow, constant::Constant, leg::Leg, market::Market, process::Process,
};

// -----------------------------------------------------------------------------
// DependencyError
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum DependencyError {
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct ComponentDependency {
    edges: HashMap<ComponentKey, HashSet<ComponentKey>>,
    order: Vec<ComponentKey>,
}

//
// display, serde
//
impl ComponentDependency {
    /// Returns the edges of the dependency graph.
    /// The key is the dependent component, and the value is the set of components which the key depends on.
    #[inline]
    pub fn edges(&self) -> &HashMap<ComponentKey, HashSet<ComponentKey>> {
        &self.edges
    }

    /// Returns the components ordered by the number of components which depends on it.
    /// So the first element is not depended by any other components
    /// and the last element is depended by many other components, including indirect dependency.
    #[inline]
    pub fn ordered_nodes(&self) -> &[ComponentKey] {
        &self.order
    }
}

// -----------------------------------------------------------------------------
// ValueOrId
//
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum ValueOrId<T> {
    Value(T),
    Id(String),
}

//
// methods
//
impl<T> ComponentField for ValueOrId<T> {
    #[inline]
    fn depends_on(&self) -> impl IntoIterator<Item = &str> {
        enum Either<L, R> {
            Left(L),
            Right(R),
        }
        impl<L, R> Iterator for Either<L, R>
        where
            L: Iterator,
            R: Iterator<Item = L::Item>,
        {
            type Item = L::Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Either::Left(l) => l.next(),
                    Either::Right(r) => r.next(),
                }
            }
        }
        match self {
            ValueOrId::Value(_) => Either::Left([].into_iter()),
            ValueOrId::Id(id) => Either::Right(id.depends_on().into_iter()),
        }
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

    type DateTime = DateWithTag;
    type DayCount = DayCountSymbol;
    type Calendar = CalendarSymbol;

    type CashflowRef = String;
    type LegRef = String;
    type MarketRef = String;
    type ProcessRef = String;

    type CompoundingConvention = String;
}

// -----------------------------------------------------------------------------
// ProductData
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProductData<V = f64> {
    pub collateral: Collateral,
    pub constants: HashMap<String, Constant>,
    pub markets: HashMap<String, Market>,
    pub processes: HashMap<String, Process<VariableTypesForData<V>>>,
    pub cashflows: HashMap<String, Cashflow<VariableTypesForData<V>>>,
    pub legs: HashMap<String, Leg<VariableTypesForData<V>>>,
}

//
// methods
//
impl<V> ProductData<V> {
    pub fn dependency(&self) -> Result<ComponentDependency, DependencyError> {
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
                    return Err(DependencyError::MissingRequiredDependency {
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
            return Err(DependencyError::NoRootComponent);
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
            return Err(DependencyError::Circular { at });
        }

        Ok(ComponentDependency {
            edges,
            order: sorted,
        })
    }
}
