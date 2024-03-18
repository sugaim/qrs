use core::panic;
use std::collections::HashMap;

use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Meta, WhereClause};

pub fn derive_has_dependency(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match &input.data {
        Data::Struct(_) => impl_for_struct(&input),
        Data::Enum(_) => impl_for_enum(&input),
        Data::Union(_) => abort!(input, "Component can only be derived for structs and enums"),
    }
}

fn impl_for_enum(input: &DeriveInput) -> proc_macro::TokenStream {
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

    let module_name = quote!(crate::product::general::core);

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
    let variant_iter_impl = generate_variant_iter(variants.len());
    let variant_items = generate_variant_items(variants.len());
    quote! {
        impl #impl_generics #module_name ::HasDependency for #name #ty_generics #where_clause {
            #[inline]
            fn depends_on(&self) -> impl IntoIterator<Item = (
                &str,
                #module_name ::ComponentCategory
            )> {
                #variant_iter_impl

                let res = match self {
                    #(
                        Self :: #variants (variant) => VariantIntoIter:: #variant_items (
                            #module_name ::HasDependency::depends_on(variant)
                        ),
                    )*
                };
                res
            }
        }
    }
    .into()
}

fn impl_for_struct(input: &DeriveInput) -> proc_macro::TokenStream {
    let Data::Struct(data) = &input.data else {
        panic!("unexpected");
    };
    let fields: HashMap<_, _> = data
        .fields
        .iter()
        .filter_map(|f| {
            let field_attrib = ParsedFieldAttribute::parse(&f.attrs);
            let field_attrib = field_attrib.unwrap_or_else(|e| abort!(f, e));
            field_attrib.map(|at| (f.ident.as_ref().unwrap().clone(), (&f.ty, at)))
        })
        .collect();

    let module_name = quote!(crate::product::general::core);

    let mut field_names = Vec::default();
    let mut field_types = Vec::default();
    let mut field_categories = Vec::default();
    let mut sub_component_types = Vec::default();
    let mut sub_components = Vec::default();
    for (field, (ty, attrib)) in &fields {
        match attrib {
            ParsedFieldAttribute::SubComponent => {
                sub_component_types.push(ty);
                sub_components.push(field);
            }
            ParsedFieldAttribute::Field(cat) => {
                field_names.push(field);
                field_types.push(ty);
                field_categories.push(quote!(#module_name ::ComponentCategory::#cat));
            }
        }
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = {
        let mut res = where_clause.cloned().unwrap_or(WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
        for ty in &field_types {
            res.predicates
                .push(syn::parse_quote!(#ty: #module_name ::ComponentField));
        }
        for ty in &sub_component_types {
            res.predicates
                .push(syn::parse_quote!(#ty: #module_name ::HasDependency));
        }
        res
    };
    quote! {
        impl #impl_generics #module_name ::HasDependency for #name #ty_generics #where_clause {
            #[inline]
            fn depends_on(&self) -> impl IntoIterator<Item = (
                &str,
                #module_name ::ComponentCategory,
            )> {
                let res = [].into_iter();
                #(
                    let res = res.chain(
                        #module_name ::HasDependency::depends_on(&self.#sub_components)
                    );
                )*
                #(
                    let res = res.chain(
                        #module_name ::ComponentField::depends_on(&self.#field_names)
                            .into_iter()
                            .map(|s| (s, #field_categories))
                    );
                )*
                res
            }
        }
    }
    .into()
}
enum ParsedFieldAttribute {
    SubComponent,
    Field(syn::Ident),
}

impl ParsedFieldAttribute {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let component_attr = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("has_dependency"));
        for attr in component_attr {
            if let Meta::Path(_) = &attr.meta {
                if res.is_some() {
                    abort!(attr, "Multiple `has_dependency` attributes are not allowed");
                }
                res = Some(ParsedFieldAttribute::SubComponent);
                continue;
            }
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("ref_category") {
                    let Ok(cat) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                        abort!(
                            attr,
                            "Format expected by `ref_category` attribute is `ref_category = \"...\""
                        );
                    };
                    if res.is_some() {
                        abort!(attr, "Multiple `ref_category` attributes are not allowed");
                    }
                    res = Some(ParsedFieldAttribute::Field(cat.parse()?));
                }
                else {
                    abort!(attr, "Unknown attribute. Expected `ref_category` as the `has_dependency(...)` attribute on the field")
                }
                Ok(())
            })?;
        }
        Ok(res)
    }
}

fn generate_variant_items(n: usize) -> Vec<syn::Ident> {
    (0..n)
        .map(|i| syn::Ident::new(&format!("T{}", i), proc_macro2::Span::call_site()))
        .collect()
}

fn generate_variant_iter(n: usize) -> proc_macro2::TokenStream {
    let types = generate_variant_items(n);
    quote! {
        enum VariantIter<#(#types),*> {
            #(
                #types(#types),
            )*
        }

        impl<Item, #(#types),*> Iterator for VariantIter<#(#types),*>
        where
            #(
                #types: Iterator<Item = Item>,
            )*
        {
            type Item = Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    #(
                        Self :: #types (iter) => iter.next(),
                    )*
                }
            }
        }

        enum VariantIntoIter<#(#types),*> {
            #(
                #types(#types),
            )*
        }
        impl<Item, #(#types),*> IntoIterator for VariantIntoIter<#(#types),*>
        where
            #(
                #types: IntoIterator<Item = Item>,
            )*
        {
            type Item = Item;
            type IntoIter = VariantIter<#(#types::IntoIter),*>;

            fn into_iter(self) -> Self::IntoIter {
                match self {
                    #(
                        Self :: #types (iter) => VariantIter::#types(iter.into_iter()),
                    )*
                }
            }
        }
    }
}
