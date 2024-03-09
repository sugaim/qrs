use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Ident};

pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let root_attrib = ParsedRootAttribute::parse(&input.attrs).unwrap_or_else(|e| abort!(input, e));
    let Data::Struct(data) = &input.data else {
        abort!(input, "Component can only be derived for structs");
    };

    let fields: HashMap<_, _> = data
        .fields
        .iter()
        .filter_map(|f| {
            let field_attrib =
                ParsedFieldAttribute::parse(&f.attrs).unwrap_or_else(|e| abort!(f, e));
            field_attrib.map(|at| (f.ident.as_ref().unwrap().clone(), at))
        })
        .collect();

    let value_type_impl =
        extract_value_type_impl(&input, &root_attrib, &fields).unwrap_or_else(|e| abort!(input, e));

    let Some(category) = root_attrib.category else {
        abort!(input, "Category of the component is not specified. please set `category` attribute on the root struct");
    };

    let module_name = if root_attrib.is_from_qrs_finance {
        quote!(crate::products::general)
    } else {
        quote!(qrs_finance::products::general)
    };

    let mut field_names = Vec::default();
    let mut categories = Vec::default();
    let mut value_types = Vec::default();
    for (field, attrib) in &fields {
        let ParsedFieldAttribute::ComponentField {
            value_type,
            category,
        } = &attrib
        else {
            continue;
        };
        field_names.push(field);
        categories.push(
            category
                .as_ref()
                .map(|c| quote!(#module_name ::ComponentCategory::#c))
                .unwrap_or_else(|| abort!(field, "Category of the component is not specified. please set `category` attribute on the field")));
        value_types.push(
            value_type
                .as_ref()
                .map(|v| quote!(#module_name ::ValueType:: #v))
                .unwrap_or_else(|| quote!(#module_name ::Component::value_type(self))),
        );
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics #module_name ::Component for #name #ty_generics #where_clause {
            #[inline]
            fn category(&self) -> #module_name ::ComponentCategory {
                #module_name ::ComponentCategory::#category
            }

            #[inline]
            fn value_type(&self) -> #module_name ::ValueType {
                #value_type_impl
            }

            #[inline]
            fn depends_on(&self) -> impl IntoIterator<Item = (
                &str,
                #module_name ::ComponentCategory,
                #module_name ::ValueType
            )> {
                let res = [].into_iter();
                #(
                    let res = res.chain(
                        #module_name ::ComponentField::depends_on(&self.#field_names)
                            .into_iter()
                            .map(|s| (s, #categories, #value_types))
                    );
                )*
                res
            }
        }
    };
    TokenStream::from(expanded)
}

#[derive(Default)]
struct ParsedRootAttribute {
    value_type: Option<syn::Ident>,
    category: Option<syn::Ident>,
    is_from_qrs_finance: bool,
}

impl ParsedRootAttribute {
    fn parse(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut res = ParsedRootAttribute::default();
        let component_attr = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("component"));
        for attr in component_attr {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("value_type") {
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
                else if m.path.is_ident("category") {
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
                else if m.path.is_ident("_use_from_qrs_finance") {
                    res.is_from_qrs_finance = true;
                }
                else {
                    abort!(attr, "Unknown attribute. Expected `component(value_type = ...)` or `component(category = ...)` on the root struct")
                }
                Ok(())
            })?;
        }
        Ok(res)
    }
}

enum ParsedFieldAttribute {
    ValueType,
    ComponentField {
        value_type: Option<syn::Ident>,
        category: Option<syn::Ident>,
    },
}

impl ParsedFieldAttribute {
    fn parse(attrs: &[Attribute]) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let component_attr = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("component"));
        for attr in component_attr {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("value_type") {
                    if res.is_some() {
                        abort!(attr, "Multiple `value_type` attributes are not allowed");
                    }
                    res = Some(ParsedFieldAttribute::ValueType);
                    return Ok(());
                }
                if m.path.is_ident("field") {
                    return m.parse_nested_meta(|m| {
                        if m.path.is_ident("value_type") {
                            let Ok(vt) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                                abort!(
                                    attr,
                                    "Format expected by `value_type` attribute is `value_type = \"...\""
                                );
                            };
                            if let Some(ParsedFieldAttribute::ComponentField { value_type, .. }) = res.as_mut() {
                                if value_type.is_some() {
                                    abort!(attr, "Multiple `value_type` attributes are not allowed");
                                }
                                *value_type = Some(vt.parse()?);
                            }
                            else {
                                res = Some(ParsedFieldAttribute::ComponentField { value_type: Some(vt.parse()?), category: None});
                            }
                        }
                        else if m.path.is_ident("category") {
                            let Ok(cat) = m.value().and_then(|v| v.parse::<syn::LitStr>()) else {
                                abort!(
                                    attr,
                                    "Format expected by `category` attribute is `category = \"...\""
                                );
                            };
                            if let Some(ParsedFieldAttribute::ComponentField { category, .. }) = res.as_mut() {
                                if category.is_some() {
                                    abort!(attr, "Multiple `category` attributes are not allowed");
                                }
                                *category = Some(cat.parse()?);
                            }
                            else {
                                res = Some(ParsedFieldAttribute::ComponentField { value_type: None, category: Some(cat.parse()?)});
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

fn extract_value_type_impl(
    input: &DeriveInput,
    root: &ParsedRootAttribute,
    fields: &HashMap<Ident, ParsedFieldAttribute>,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let mut value_type_field = None;
    for (field, attrib) in fields {
        if let ParsedFieldAttribute::ValueType = attrib {
            if value_type_field.is_some() {
                abort!(field, "Multiple `value_type` attributes are not allowed");
            }
            value_type_field = Some(field);
        }
    }

    let res = match (value_type_field, &root.value_type) {
        (Some(_), Some(_)) => {
            abort!(
                input,
                "Multiple `value_type` attributes are not allowed. Please set `value_type` attribute on the root struct or on a field"
            );
        }
        (Some(field), None) => quote! { self. #field },
        (None, Some(value_type)) => quote! { ValueType:: #value_type },
        (None, None) => {
            abort!(input, "value type of the component is not specified. please set `value_type` attribute on the root struct or on a field");
        }
    };
    Ok(res)
}
