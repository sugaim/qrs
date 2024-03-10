#[cfg(feature = "derive")]
pub use qrs_finance_derive::Component;

pub mod components;
mod core;
mod product;

pub use core::{
    Component, ComponentCategory, ComponentField, ComponentGraph, ComponentKey, DependencyError,
    ValueOrId, ValueType, VariableTypes, VariableTypesForParse,
};
pub use product::GeneralProduct;
