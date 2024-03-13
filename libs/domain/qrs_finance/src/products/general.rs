#[cfg(feature = "derive")]
pub use qrs_finance_derive::Component;

mod components;
pub mod core;
mod product;

pub use components::{cashflow, constant, leg, market, process};
pub use product::{GeneralProduct, GeneralProductBuilder, ProductData, VariableTypesForData};
