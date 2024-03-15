mod data;

#[allow(clippy::module_inception)]
mod product;

pub use data::{ContractData, ProductData, VariableTypesForData};
pub use product::{BuildProduct, Product, ProductBuilder, VariableTypesExpanded};
