use std::collections::HashMap;

use crate::products::general::components::{constant::Constant, process::Process};

use super::ComponentKey;

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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentGraph {
    pub constants: HashMap<String, Constant>,
    pub process: HashMap<String, Process>,
}
