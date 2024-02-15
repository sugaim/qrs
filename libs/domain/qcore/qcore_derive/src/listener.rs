use core::panic;

use either::Either;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parser, parse_quote, punctuated::Punctuated, Attribute, Data, DeriveInput, Fields, Token,
};

pub fn derive_listener(input: TokenStream) -> TokenStream {
    let class = syn::parse_macro_input!(input as DeriveInput);
    let base_field = parse_listener_attrib_to_specify_field(&class.attrs);
    let root = quote!(crate);
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();

    let mut where_clause = match where_clause {
        Some(wc) => {
            let mut wc = wc.clone();
            let new_statement = match wc.predicates.len() {
                0 => parse_quote!(where Self: 'static + Send + Sync),
                _ => parse_quote!(Self: 'static + Send + Sync),
            };
            wc.predicates.push(new_statement);
            wc
        }
        None => parse_quote!(where Self: 'static + Send + Sync),
    };

    // structure dependent processing, validation
    let (field, field_name) = match (&class.data, &base_field) {
        (_, None) => panic!(
            "#[derive(Listener)] must be used with #[listener(transparent = ...)] to specify field to define listener trait"
        ),
        (Data::Enum(_), _) => panic!("#[derive(Listener)] can only be used on struct"),
        (Data::Union(_), _) => panic!("#[derive(Listener)] can only be used on struct"),
        (Data::Struct(s), Some(Either::Left(n))) => {
            // tuple
            match &s.fields {
                Fields::Unnamed(fs) => {
                    if fs.unnamed.len() <= *n as usize {
                        panic!("field index is out of range");
                    }
                    (&fs.unnamed[*n as usize], syn::Index::from(*n as usize).into_token_stream())
                }
                _ => panic!("field index is specified but the struct is not tuple"),
            }
        }
        (Data::Struct(s), Some(Either::Right(name))) => {
            // named
            match &s.fields {
                Fields::Named(fs) => {
                    let Some(field) = fs.named.iter().find(|f| f.ident.as_ref() == Some(name)) else {
                        panic!("field named {name} is specified as base listener but it is not found");
                    };
                    (field, quote!(#name))
                }
                _ => panic!("field name is specified but the struct is not named"),
            }
        }
    };
    let ty = &field.ty;
    where_clause
        .predicates
        .push(parse_quote!(#ty: #root ::datasrc::Listener));

    let name = class.ident;
    quote!(
        impl #impl_generics #root ::datasrc::Listener for #name #ty_generics #where_clause {
            #[inline]
            fn id(&self) -> #root ::datasrc::NodeId {
                self. #field_name .id()
            }

            #[inline]
            fn listen(&mut self, id: &#root ::datasrc::NodeId, state: &#root ::datasrc::StateId) {
                self. #field_name .listen(id, state)
            }
        }
    )
    .into()
}

fn parse_listener_attrib_to_specify_field(attribs: &[Attribute]) -> Option<Either<u16, Ident>> {
    let attribs: Vec<_> = attribs
        .iter()
        .filter_map(|attr| attr.meta.require_list().ok())
        .filter(|l| l.path.is_ident("listener"))
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
        _ => panic!("Multiple `listener(transparent = ...)` attributes are not allowed"),
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
        _ => panic!("only integer or string is allowed as #[listener(transparent = ...)]"),
    }
}
