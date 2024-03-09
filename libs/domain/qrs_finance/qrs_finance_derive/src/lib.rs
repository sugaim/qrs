mod products;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_error]
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    products::derive_component(input)
}
