mod product;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_error]
#[proc_macro_derive(HasDependency, attributes(has_dependency))]
pub fn derive_has_dependency(input: TokenStream) -> TokenStream {
    product::derive_has_dependency(input)
}

#[proc_macro_error]
#[proc_macro_derive(Component, attributes(component, has_dependency))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    product::derive_component(input)
}
