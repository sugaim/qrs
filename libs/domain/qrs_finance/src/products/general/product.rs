mod data;
#[allow(clippy::module_inception)]
mod product;

pub use data::{ProductData, VariableTypesForData};

pub use product::{GeneralProduct, GeneralProductBuilder};
