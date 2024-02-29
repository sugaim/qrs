use proc_macro_error::proc_macro_error;

mod debug_tree;

#[proc_macro_error]
#[proc_macro_derive(DebugTree, attributes(debug_tree))]
pub fn derive_debug_tree(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    debug_tree::derive_debug_tree(input)
}
