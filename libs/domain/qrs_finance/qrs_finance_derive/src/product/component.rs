use core::panic;

use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, WhereClause};

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut result = super::derive_has_dependency(input.clone());
    let input = parse_macro_input!(input as DeriveInput);
    let component = match &input.data {
        Data::Struct(_) => impl_for_struct(&input),
        Data::Enum(_) => impl_for_enum(&input),
        Data::Union(_) => abort!(input, "Component can only be derived for structs and enums"),
    };
    result.extend(component);
    result
}

fn impl_for_enum(input: &DeriveInput) -> TokenStream {
    let root_attrib = ParsedRootAttribute::parse(&input.attrs).unwrap_or_else(|e| abort!(input, e));
    let Data::Enum(data) = &input.data else {
        panic!("unexpected");
    };
    for variant in &data.variants {
        if variant.fields.len() != 1 {
            abort!(
                variant,
                "Component can only be derived for new type variants"
            );
        }
        if variant.fields.iter().next().unwrap().ident.is_some() {
            abort!(
                variant.fields.iter().next().unwrap(),
                "Component can only be derived for new type variants"
            );
        }
    }
    let (variants, types): (Vec<_>, Vec<_>) = data
        .variants
        .iter()
        .map(|v| (&v.ident, &v.fields.iter().next().unwrap().ty))
        .unzip();

    let module_name = root_attrib.module_name();

    if root_attrib.category.is_some() {
        abort!(input, "Category of the component is not allowed for enum. Category is selected based on the variant");
    }
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = {
        let mut res = where_clause.cloned().unwrap_or(WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
        for ty in &types {
            res.predicates
                .push(syn::parse_quote!(#ty: #module_name ::Component));
        }
        res
    };
    quote! {
        impl #impl_generics #module_name ::Component for #name #ty_generics #where_clause {
            #[inline]
            fn category(&self) -> #module_name ::ComponentCategory {
                match self {
                    #(
                        Self :: #variants (variant) => #module_name ::Component::category(variant),
                    )*
                }
            }
        }
    }
    .into()
}

fn impl_for_struct(input: &DeriveInput) -> TokenStream {
    let root_attrib = ParsedRootAttribute::parse(&input.attrs).unwrap_or_else(|e| abort!(input, e));

    let module_name = root_attrib.module_name();
    let Some(category) = &root_attrib.category else {
        abort!(input, "Category of the component is not specified. please set `category` attribute on the root struct");
    };
    let category = quote!(#module_name ::ComponentCategory::#category);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = {
        let mut res = where_clause.cloned().unwrap_or(WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
        res.predicates
            .push(syn::parse_quote!(Self: #module_name ::HasDependency));
        res
    };
    quote! {
        impl #impl_generics #module_name ::Component for #name #ty_generics #where_clause {
            #[inline]
            fn category(&self) -> #module_name ::ComponentCategory {
                #category
            }
        }
    }
    .into()
}

#[derive(Default)]
struct ParsedRootAttribute {
    category: Option<syn::Ident>,
}

impl ParsedRootAttribute {
    fn module_name(&self) -> proc_macro2::TokenStream {
        quote!(crate::product::general::core)
    }

    fn parse(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut res = ParsedRootAttribute::default();
        let component_attr = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("component"));
        for attr in component_attr {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("category") {
                    if res.category.is_some() {
                        abort!(attr, "Multiple `category` attributes are not allowed");
                    }
                    let Ok(category) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                        abort!(
                            attr,
                            "Format expected by `category` attribute is `category = \"...\""
                        );
                    };
                    res.category = Some(category.parse()?);
                }
                else {
                    abort!(attr, "Unknown attribute. Expected `component(category = ...)` on the root struct")
                }
                Ok(())
            })?;
        }
        Ok(res)
    }
}
