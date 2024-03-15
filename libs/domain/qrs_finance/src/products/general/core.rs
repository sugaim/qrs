mod component;
mod component_field;
mod value_or_id;

pub(crate) use component::Component;

pub use component::{ComponentCategory, ComponentKey, VariableTypes};
pub use component_field::ComponentField;
pub use value_or_id::ValueOrId;
