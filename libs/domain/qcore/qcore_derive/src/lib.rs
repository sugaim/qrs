use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Fields};

#[proc_macro_attribute]
pub fn node_transparent(_attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let class = {
        let item = item.clone();
        syn::parse_macro_input!(item as DeriveInput)
    };
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();

    let internal = match &class.data {
        Data::Struct(s) => match &s.fields {
            Fields::Unnamed(fs) => match fs.unnamed.len() {
                1 => fs.unnamed.first().unwrap().clone(),
                _ => panic!("node_transparent can only be used on structs with a single field"),
            },
            _ => panic!("node_transparent can only be used on tuple with a single field"),
        },
        _ => panic!("node_transparent can only be used on structs"),
    };
    let Some(ty) = wrapped_type_of_arc(&internal.ty) else {
        panic!("node_transparent can only be used on structs with a single field of type Arc<T>");
    };
    let crate_name = quote!(crate);
    let where_clause = match where_clause {
        Some(wc) => {
            let mut wc = wc.clone();
            wc.predicates
                .push(parse_quote!(where #ty: #crate_name ::datasrc::Node));
            wc
        }
        None => parse_quote!(where #ty: #crate_name ::datasrc::Node),
    };

    let name = class.ident;
    let node_impl: TokenStream = quote!(
        impl #impl_generics #crate_name ::datasrc::Node for #name #ty_generics #where_clause {
            #[inline]
            fn id(&self) -> #crate_name ::datasrc::NodeId {
                self.0.id()
            }

            #[inline]
            fn tree(&self) -> #crate_name ::datasrc::Tree {
                self.0.tree()
            }

            #[inline]
            fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn #crate_name ::datasrc::Node>) -> #crate_name ::datasrc::NodeStateId {
                self.0.accept_subscriber(subscriber)
            }

            #[inline]
            fn remove_subscriber(&self, subscriber: &#crate_name ::datasrc::NodeId) {
                self.0.remove_subscriber(subscriber)
            }
            #[inline]
            fn accept_state(&self, id: &#crate_name ::datasrc::NodeId, state: &#crate_name ::datasrc::NodeStateId) {
                self.0.accept_state(id, state)
            }
        }
    ).into();
    item.extend(node_impl);
    item
}

fn wrapped_type_of_arc(ty: &syn::Type) -> Option<syn::Type> {
    let seg = match ty {
        syn::Type::Path(tp) => match tp.path.segments.last() {
            Some(seg) => seg,
            None => return None,
        },
        _ => return None,
    };
    if seg.ident != "Arc" {
        return None;
    }
    let ty = match &seg.arguments {
        syn::PathArguments::AngleBracketed(ab) => match ab.args.first() {
            Some(arg) => match arg {
                syn::GenericArgument::Type(ty) => ty,
                _ => return None,
            },
            None => return None,
        },
        _ => return None,
    };
    Some(ty.clone())
}
