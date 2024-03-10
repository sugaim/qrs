use std::collections::HashMap;

use derivative::Derivative;

use crate::products::general::components::{constant::Constant, market::Market, process::Process};

use super::{ComponentKey, VariableTypes};

// -----------------------------------------------------------------------------
// DependencyError
//
#[derive(Debug, thiserror::Error)]
pub enum DependencyError {
    #[error("{} is required by {} but missing", .required, .by)]
    Missing {
        required: ComponentKey,
        by: ComponentKey,
    },
}

// -----------------------------------------------------------------------------
// ComponentGraph
//
#[derive(Debug, Clone, Default, Derivative)]
pub struct ComponentGraph<Ts: VariableTypes> {
    pub constants: HashMap<String, Constant>,
    pub markets: HashMap<String, Market>,
    pub process: HashMap<String, Process<Ts>>,
}
