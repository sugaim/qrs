use core::panic;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, WhereClause};

pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match &input.data {
        Data::Struct(_) => impl_for_struct(&input),
        Data::Enum(_) => impl_for_enum(&input),
        Data::Union(_) => abort!(input, "Component can only be derived for structs and enums"),
    }
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
    let variant_iter_impl = generate_variant_iter(variants.len());
    let variant_items = generate_variant_items(variants.len());
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

            #[inline]
            fn depends_on(&self) -> impl IntoIterator<Item = (
                &str,
                #module_name ::ComponentCategory
            )> {
                #variant_iter_impl

                let res = match self {
                    #(
                        Self :: #variants (variant) => VariantIntoIter:: #variant_items (
                            #module_name ::Component::depends_on(variant)
                        ),
                    )*
                };
                res
            }
        }
    }
    .into()
}

fn impl_for_struct(input: &DeriveInput) -> TokenStream {
    let root_attrib = ParsedRootAttribute::parse(&input.attrs).unwrap_or_else(|e| abort!(input, e));

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

    let module_name = root_attrib.module_name();
    let Some(category) = &root_attrib.category else {
        abort!(input, "Category of the component is not specified. please set `category` attribute on the root struct");
    };
    let category = if let Some(vt) = root_attrib.value_type {
        quote!(#module_name ::ComponentCategory::#category (#module_name ::ValueType::#vt))
    } else {
        quote!(#module_name ::ComponentCategory::#category)
    };

    let mut field_names = Vec::default();
    let mut field_types = Vec::default();
    let mut categories = Vec::default();
    for (field, (ty, attrib)) in &fields {
        let ParsedFieldAttribute {
            category,
            value_type,
        } = &attrib;
        field_names.push(field);
        field_types.push(ty);
        categories.push(
            category
                .as_ref()
                .map(|c| {
                    if let Some(vt) = value_type {
                        quote!(#module_name ::ComponentCategory::#c (#module_name ::ValueType::#vt))
                    } else {
                        quote!(#module_name ::ComponentCategory::#c)
                    }
                })
                .unwrap_or_else(|| abort!(field, "Category of the component is not specified. please set `category` attribute on the field"))
        );
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
        res
    };
    quote! {
        impl #impl_generics #module_name ::Component for #name #ty_generics #where_clause {
            #[inline]
            fn category(&self) -> #module_name ::ComponentCategory {
                #category
            }

            #[inline]
            fn depends_on(&self) -> impl IntoIterator<Item = (
                &str,
                #module_name ::ComponentCategory,
            )> {
                let res = [].into_iter();
                #(
                    let res = res.chain(
                        #module_name ::ComponentField::depends_on(&self.#field_names)
                            .into_iter()
                            .map(|s| (s, #categories))
                    );
                )*
                res
            }
        }
    }
    .into()
}

#[derive(Default)]
struct ParsedRootAttribute {
    category: Option<syn::Ident>,
    value_type: Option<syn::Ident>,
    is_from_qrs_finance: bool,
}

impl ParsedRootAttribute {
    fn module_name(&self) -> proc_macro2::TokenStream {
        if self.is_from_qrs_finance {
            quote!(crate::products::general::core)
        } else {
            quote!(qrs_finance::products::general::core)
        }
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
                else if m.path.is_ident("value_type") {
                    if res.value_type.is_some() {
                        abort!(attr, "Multiple `value_type` attributes are not allowed");
                    }
                    let Ok(value_type) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                        abort!(
                            attr,
                            "Format expected by `value_type` attribute is `value_type = \"...\""
                        );
                    };
                    res.value_type = Some(value_type.parse()?);
                }
                else if m.path.is_ident("_use_from_qrs_finance") {
                    res.is_from_qrs_finance = true;
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

struct ParsedFieldAttribute {
    category: Option<syn::Ident>,
    value_type: Option<syn::Ident>,
}

impl ParsedFieldAttribute {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let component_attr = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("component"));
        for attr in component_attr {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("field") {
                    return m.parse_nested_meta(|m| {
                        if m.path.is_ident("category") {
                            let Ok(cat) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                                abort!(
                                    attr,
                                    "Format expected by `category` attribute is `category = \"...\""
                                );
                            };
                            if let Some(ParsedFieldAttribute { category, .. }) = res.as_mut() {
                                if category.is_some() {
                                    abort!(attr, "Multiple `category` attributes are not allowed");
                                }
                                *category = Some(cat.parse()?);
                            }
                            else {
                                res = Some(ParsedFieldAttribute { category: Some(cat.parse()?), value_type: None });
                            }
                        }
                        else if m.path.is_ident("value_type") {
                            let Ok(vt) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                                abort!(
                                    attr,
                                    "Format expected by `value_type` attribute is `value_type = \"...\""
                                );
                            };
                            if let Some(ParsedFieldAttribute { value_type, .. }) = res.as_mut() {
                                if value_type.is_some() {
                                    abort!(attr, "Multiple `value_type` attributes are not allowed");
                                }
                                *value_type = Some(vt.parse()?);
                            }
                            else {
                                res = Some(ParsedFieldAttribute { category: None, value_type: Some(vt.parse()?) });
                            }
                        }
                        else {
                            abort!(attr, "Unknown attribute. Expected `value_type` or `category` as the `component(...)` attribute on the field")
                        }
                        Ok(())
                    });
                }
                abort!(attr, "Unknown attribute. Expected `value_type` or `field` as the `component(...)` attribute on the field")
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
