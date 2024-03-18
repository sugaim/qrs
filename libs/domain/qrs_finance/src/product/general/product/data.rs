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
    product::{
        core::{Collateral, InArrears},
        general::{
            cashflow::CashflowFixing,
            core::{Component, ComponentKey, HasDependency, ValueLess, ValueOrId, VariableTypes},
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
                    id: from.clone().into(),
                };
                let node = edges.entry(key).or_default();
                node.extend(
                    comp.depends_on()
                        .into_iter()
                        .map(|(to, cat)| ComponentKey { cat, id: to.into() }),
                );
            }
            for (from, comp) in &self.markets {
                let key = ComponentKey {
                    cat: comp.category(),
                    id: from.clone().into(),
                };
                let node = edges.entry(key).or_default();
                node.extend(
                    comp.depends_on()
                        .into_iter()
                        .map(|(to, cat)| ComponentKey { cat, id: to.into() }),
                );
            }
            for (from, comp) in &self.processes {
                let key = ComponentKey {
                    cat: comp.category(),
                    id: from.clone().into(),
                };
                let node = edges.entry(key).or_default();
                node.extend(
                    comp.depends_on()
                        .into_iter()
                        .map(|(to, cat)| ComponentKey { cat, id: to.into() }),
                );
            }
            for (from, comp) in &self.cashflows {
                let key = ComponentKey {
                    cat: comp.category(),
                    id: from.clone().into(),
                };
                let node = edges.entry(key).or_default();
                node.extend(
                    comp.depends_on()
                        .into_iter()
                        .map(|(to, cat)| ComponentKey { cat, id: to.into() }),
                );
            }
            for (from, comp) in &self.legs {
                let key = ComponentKey {
                    cat: comp.category(),
                    id: from.clone().into(),
                };
                let node = edges.entry(key).or_default();
                node.extend(
                    comp.depends_on()
                        .into_iter()
                        .map(|(to, cat)| ComponentKey { cat, id: to.into() }),
                );
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

// =============================================================================
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use maplit::{hashmap, hashset};
    use rstest::rstest;

    use super::*;
    use crate::product::general::core::{ComponentCategory, ComponentKey};

    fn testdata_root() -> PathBuf {
        let mut res = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        res.push("testdata");
        res.push("product");
        res.push("general");
        res
    }

    use ComponentCategory::*;

    fn key(cat: ComponentCategory, id: &str) -> ComponentKey {
        ComponentKey { cat, id: id.into() }
    }

    #[rstest]
    #[case(
        "from_leg",
        hashmap! {
            key(Cashflow, "cpn1") => Default::default(),
            key(Cashflow, "cpn2") => Default::default(),
            key(Cashflow, "cpn3") => Default::default(),
            key(Leg, "leg1") => hashset! { key(Cashflow, "cpn2"), },
            key(Leg, "leg2") => hashset! { key(Cashflow, "cpn1"), key(Cashflow, "cpn3"), }
        },
    )]
    #[case(
        "from_cf",
        hashmap! {
            key(Constant, "notional") => Default::default(),
            key(Constant, "daycount") => Default::default(),
            key(Constant, "convention") => Default::default(),
            key(Market, "tona") => Default::default(),
            key(Cashflow, "cpn1") => hashset! { key(Constant, "notional"), },
            key(Cashflow, "cpn2") => hashset! { key(Constant, "notional"), key(Constant, "daycount"), },
            key(Cashflow, "cpn3") => hashset! { key(Market, "tona"), key(Constant, "convention"), },
        },
    )]
    #[case(
        "from_proc",
        hashmap! {
            key(Constant, "cnst1") => Default::default(),
            key(Constant, "cnst2") => Default::default(),
            key(Market, "tona") => Default::default(),
            key(Process, "proc1") => hashset! { key(Market, "tona"), },
            key(Process, "proc2") => hashset! { key(Constant, "cnst1"), key(Constant, "cnst2"), }
        },
    )]
    fn test_dependency_tracked(
        #[case] path: &str,
        #[case] expected: HashMap<ComponentKey, HashSet<ComponentKey>>,
    ) {
        // dependency is collected component by component.
        // so this test tries to check the dependency is correctly collected.
        // (so, test case is named 'from_*')
        let path = testdata_root().join(format!("contract.dep.{}.yaml", path));
        let yaml = std::fs::read_to_string(path).unwrap();
        let data: ContractData = serde_yaml::from_str(&yaml).unwrap();

        let dep = data._dependency().unwrap().edges;

        assert_eq!(dep, expected);
    }

    #[rstest]
    fn test_missing_dependency() {
        let path = testdata_root().join("contract.err.missing_dep.yaml");
        let yaml = std::fs::read_to_string(path).unwrap();
        let data: ContractData = serde_yaml::from_str(&yaml).unwrap();

        let dep = data._dependency();

        assert!(dep.is_err());
        let err = dep.unwrap_err();
        assert_eq!(
            err,
            _DependencyError::MissingRequiredDependency {
                required: key(Market, "tona"),
                by: key(Cashflow, "cpn3"),
            }
        )
    }
}
