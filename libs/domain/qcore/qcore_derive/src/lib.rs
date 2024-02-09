use core::panic;

use either::Either;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    parse::Parser, parse_quote, punctuated::Punctuated, Attribute, Data, DeriveInput, Fields, Token,
};

#[proc_macro_derive(Node, attributes(node))]
pub fn derive_node(input: TokenStream) -> TokenStream {
    let class = syn::parse_macro_input!(input as DeriveInput);
    let base_field = parse_node_attrib_to_specify_field(&class.attrs);
    let root = quote!(crate);
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();

    let mut where_clause = match where_clause {
        Some(wc) => {
            let mut wc = wc.clone();
            let new_statement = match wc.predicates.len() {
                0 => parse_quote!(where Self: 'static),
                _ => parse_quote!(Self: 'static),
            };
            wc.predicates.push(new_statement);
            wc
        }
        None => parse_quote!(where Self: 'static),
    };

    // structure dependent processing, validation
    let (field, field_name) = match (&class.data, &base_field) {
        (_, None) => panic!(
            "#[derive(Node)] must be used with #[node(transparent = ...)] to specify field to define Node trait"
        ),
        (Data::Enum(_), _) => panic!("#[derive(Node)] can only be used on struct"),
        (Data::Union(_), _) => panic!("#[derive(Node)] can only be used on struct"),
        (Data::Struct(s), Some(Either::Left(n))) => {
            // tuple
            match &s.fields {
                Fields::Unnamed(fs) => {
                    if fs.unnamed.len() <= *n as usize {
                        panic!("field index is out of range");
                    }
                    (&fs.unnamed[*n as usize], quote!(#n))
                }
                _ => panic!("field index is specified but the struct is not tuple"),
            }
        }
        (Data::Struct(s), Some(Either::Right(name))) => {
            // named
            match &s.fields {
                Fields::Named(fs) => {
                    let Some(field) = fs.named.iter().find(|f| f.ident.as_ref() == Some(name)) else {
                        panic!("field named {name} is specified as base node but it is not found");
                    };
                    (field, quote!(#name))
                }
                _ => panic!("field name is specified but the struct is not named"),
            }
        }
    };
    let Some(ty) = wrapped_type_of_arc(&field.ty) else {
        panic!("node_transparent can only be used on field of type Arc<T>");
    };
    where_clause
        .predicates
        .push(parse_quote!(#ty: #root ::datasrc::Node));

    let name = class.ident;
    quote!(
        impl #impl_generics #root ::datasrc::Node for #name #ty_generics #where_clause {
            #[inline]
            fn id(&self) -> #root ::datasrc::NodeId {
                self. #field_name .id()
            }

            #[inline]
            fn tree(&self) -> #root ::datasrc::Tree {
                self. #field_name .tree()
            }

            #[inline]
            fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn #root ::datasrc::Node>) -> #root ::datasrc::NodeStateId {
                self. #field_name .accept_subscriber(subscriber)
            }

            #[inline]
            fn remove_subscriber(&self, subscriber: &#root ::datasrc::NodeId) {
                self. #field_name .remove_subscriber(subscriber)
            }
            #[inline]
            fn accept_state(&self, id: &#root ::datasrc::NodeId, state: &#root ::datasrc::NodeStateId) {
                self. #field_name .accept_state(id, state)
            }
        }
    ).into()
}

#[proc_macro_attribute]
pub fn node_transparent(args: TokenStream, mut item: TokenStream) -> TokenStream {
    let class = {
        let item = item.clone();
        syn::parse_macro_input!(item as DeriveInput)
    };
    let field_spec = parse_attrib_for_specify_field(args);
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();

    let (field_name, field) = {
        let Data::Struct(s) = &class.data else {
            panic!("node_transparent can only be used on structs");
        };
        match (&s.fields, field_spec) {
            (Fields::Unnamed(fs), None) => match fs.unnamed.len() {
                1 => (quote!(0), fs.unnamed.first().unwrap()),
                _ => panic!("node_transparent can only be used on tuple with a single field"),
            },
            (Fields::Unnamed(fs), Some(Either::Left(n))) => {
                let field = fs.unnamed.iter().nth(n as usize).expect(
                    format!("{n}-th field is specified as base node but it is not found").as_str(),
                );
                (quote!(#n), field)
            }
            (Fields::Named(fs), Some(Either::Right(name))) => {
                let field = fs
                    .named
                    .iter()
                    .find(|f| f.ident.as_ref() == Some(&name))
                    .expect(
                        format!("field named {name} is specified as base node but it is not found")
                            .as_str(),
                    );
                (quote!(#name), field)
            }
            _ => panic!("mismatches between field specification and struct type"),
        }
    };

    let Some(ty) = wrapped_type_of_arc(&field.ty) else {
        panic!("node_transparent can only be used on field of type Arc<T>");
    };
    let crate_name = quote!(crate);
    let mut where_clause = match where_clause {
        Some(wc) => {
            let mut wc = wc.clone();
            let new_statement = match wc.predicates.len() {
                0 => parse_quote!(where #ty: #crate_name ::datasrc::Node),
                _ => parse_quote!(#ty: #crate_name ::datasrc::Node),
            };
            wc.predicates.push(new_statement);
            wc
        }
        None => parse_quote!(where #ty: #crate_name ::datasrc::Node),
    };
    where_clause.predicates.push(parse_quote!(Self: 'static));

    let name = class.ident;
    let node_impl: TokenStream = quote!(
        impl #impl_generics #crate_name ::datasrc::Node for #name #ty_generics #where_clause {
            #[inline]
            fn id(&self) -> #crate_name ::datasrc::NodeId {
                self. #field_name .id()
            }

            #[inline]
            fn tree(&self) -> #crate_name ::datasrc::Tree {
                self.  #field_name .tree()
            }

            #[inline]
            fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn #crate_name ::datasrc::Node>) -> #crate_name ::datasrc::NodeStateId {
                self. #field_name .accept_subscriber(subscriber)
            }

            #[inline]
            fn remove_subscriber(&self, subscriber: &#crate_name ::datasrc::NodeId) {
                self. #field_name .remove_subscriber(subscriber)
            }
            #[inline]
            fn accept_state(&self, id: &#crate_name ::datasrc::NodeId, state: &#crate_name ::datasrc::NodeStateId) {
                self. #field_name .accept_state(id, state)
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

fn parse_attrib_for_specify_field(args: TokenStream) -> Option<Either<u16, Ident>> {
    let args = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse(args)
        .unwrap();
    let arg = match args.len() {
        0 => return None,
        1 => args
            .first()
            .unwrap()
            .get_ident()
            .expect("only identifier is allowed"),
        _ => panic!("only one argument is allowed"),
    };
    if let Ok(n) = arg.to_string().parse() {
        return Some(Either::Left(n));
    }
    let arg_str = arg.to_string();
    if arg_str.chars().any(|c| !c.is_alphanumeric()) {
        panic!("only alphanumeric characters are allowed as a field name");
    }
    if arg_str.is_empty() {
        panic!("field name must not be empty");
    }
    if arg_str.chars().next().unwrap().is_numeric() {
        panic!("field name must not start with a number");
    }
    Some(Either::Right(arg.clone()))
}

fn parse_node_attrib_to_specify_field(attribs: &[Attribute]) -> Option<Either<u16, Ident>> {
    let attribs: Vec<_> = attribs
        .iter()
        .filter_map(|attr| attr.meta.require_list().ok())
        .filter(|l| l.path.is_ident("node"))
        .filter_map(|l| {
            Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated
                .parse(l.tokens.clone().into())
                .ok()
        })
        .flat_map(|args| args.into_iter())
        .filter(|arg| arg.path.is_ident("transparent"))
        .collect();

    let value = match attribs.len() {
        0 => return None,
        1 => match attribs.into_iter().next().unwrap().value {
            syn::Expr::Lit(lit) => lit.lit,
            _ => panic!("only literal is allowed"),
        },
        _ => panic!("Multiple `node(transparent = ...)` attributes are not allowed"),
    };
    match value {
        syn::Lit::Int(n) => {
            let n = n.base10_parse::<u16>().expect("only integer is allowed");
            Some(Either::Left(n))
        }
        syn::Lit::Str(s) => {
            let s = s.value();
            if s.chars().any(|c| !c.is_alphanumeric()) {
                panic!("only alphanumeric characters are allowed as a field name");
            }
            if s.is_empty() {
                panic!("field name must not be empty");
            }
            if s.chars().next().unwrap().is_numeric() {
                panic!("field name must not start with a number");
            }
            Some(Either::Right(format_ident!("{}", s)))
        }
        _ => panic!("only integer or string is allowed as #[node(transparent = ...)]"),
    }
}
