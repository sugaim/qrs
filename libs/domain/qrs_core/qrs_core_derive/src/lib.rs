use proc_macro::TokenStream;

mod listener;
mod node;
mod notifier;

#[proc_macro_derive(Listener, attributes(listener))]
pub fn derive_subscriber(input: TokenStream) -> TokenStream {
    listener::derive_listener(input)
}

#[proc_macro_derive(Notifier, attributes(notifier))]
pub fn derive_notifier(input: TokenStream) -> TokenStream {
    notifier::derive_notifier(input)
}

#[proc_macro_derive(Node, attributes(node))]
pub fn derive_node(input: TokenStream) -> TokenStream {
    node::derive_node(input)
}
