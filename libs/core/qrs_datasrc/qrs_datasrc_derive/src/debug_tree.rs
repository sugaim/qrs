use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use syn::{
    parenthesized, parse_macro_input, parse_quote, Data, DeriveInput, Fields, LitStr, Type,
    WhereClause,
};

pub fn derive_debug_tree(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let name = &input.ident;
    let root_attribs = root_attribs(&input);
    let crate_name = if root_attribs.use_from_qrs_datasrc {
        quote!(crate)
    } else {
        quote!(qrs_datasrc)
    };

    let fields = match &input.data {
        Data::Struct(input) => get_subtrees(&input.fields),
        _ => abort!(input, "DebugTree can only be derived for structs"),
    };
    let where_claues = {
        let mut wc = where_clause.cloned().unwrap_or_else(|| WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });
        for field in &fields {
            let ty = &field.ty;
            wc.predicates
                .push(parse_quote!(#ty: #crate_name ::DebugTree));
        }
        wc
    };
    let field_names = fields.iter().map(|f| &f.name);
    let tree_impl = match field_names.len() {
        0 => quote!(
            #crate_name ::TreeInfo::Leaf {
                tp: std::any::type_name::<Self>().to_string(),
                desc: #crate_name ::DebugTree::desc(self),
            }
        ),
        1 => {
            let field_name = &fields[0].name;
            quote!(
                #crate_name ::TreeInfo::Wrap {
                    tp: std::any::type_name::<Self>().to_string(),
                    child: Box::new(#crate_name ::DebugTree::debug_tree(&self.#field_name)),
                    desc: #crate_name ::DebugTree::desc(self),
                }
            )
        }
        _ => {
            let nm_strs = fields.iter().map(|f| f.name.to_string());
            quote!(
                #crate_name ::TreeInfo::Branch {
                    tp: std::any::type_name::<Self>().to_string(),
                    desc: #crate_name ::DebugTree::desc(self),
                    children: {
                        let mut children = std::collections::BTreeMap::new();
                        #(
                            children.insert(#nm_strs.to_owned(), #crate_name ::DebugTree::debug_tree(&self.#field_names));
                        )*
                        children
                    },
                }
            )
        }
    };
    let desc_impl = match root_attribs.description {
        None => quote!("no description".to_owned()),
        Some(Description::Lit(desc)) => quote!(#desc.to_string()),
        Some(Description::Field(desc)) => quote!(self.#desc.to_string()),
        Some(Description::Func(desc)) => quote!(#desc (self)),
    };
    quote!(
        impl #impl_generics #crate_name ::DebugTree for #name #ty_generics #where_claues {
            #[inline]
            fn desc(&self) -> String {
                #desc_impl
            }

            #[inline]
            fn debug_tree(&self) -> #crate_name ::TreeInfo {
                #tree_impl
            }
        }
    )
    .into()
}

fn root_attribs(input: &DeriveInput) -> RootAttribs {
    let attrbs = input
        .attrs
        .iter()
        .filter(|attr| attr.meta.path().is_ident("debug_tree"));

    let mut res = RootAttribs::default();
    for attr in attrbs {
        let res = attr.parse_nested_meta(|meta| {
            // #[debug_tree(_use_from_qrs_datasrc)]
            if meta.path.is_ident("_use_from_qrs_datasrc") {
                res.use_from_qrs_datasrc = true;
                return Ok(());
            }

            // #[debug_tree(desc = "error mapped")]
            if meta.path.is_ident("desc") {
                let Ok(desc) = meta.value().and_then(|v| v.parse::<LitStr>()) else {
                    abort!(
                        attr,
                        "Format expected by `desc` attribute is `desc = \"...\""
                    );
                };
                res.description = Some(Description::Lit(desc));
                return Ok(());
            }

            // #[debug_tree(desc_field = "desc")]
            if meta.path.is_ident("desc_field") {
                if res.description.is_some() {
                    abort!(
                        attr,
                        "Multiple `desc`, `desc_field` or `desc_func` attributes are not allowed"
                    );
                }
                let Ok(desc) = meta.value().and_then(|v| v.parse::<LitStr>()) else {
                    abort!(
                        attr,
                        "Format expected by `desc_field` attribute is `desc_field = \"field_name\""
                    );
                };
                res.description = Some(Description::Field(desc.parse()?));
                return Ok(());
            }

            // #[debug_tree(desc_func = "function_name")]
            if meta.path.is_ident("desc_func") {
                if res.description.is_some() {
                    abort!(
                        attr,
                        "Multiple `desc`, `desc_field` or `desc_func` attributes are not allowed"
                    );
                }
                let Ok(desc) = meta.value().and_then(|v| v.parse::<LitStr>()) else {
                    abort!(
                        attr,
                        "Format expected by `desc_func` attribute is `desc_func = \"function_name\""
                    );
                };
                res.description = Some(Description::Func(desc.parse()?));
                return Ok(());
            }

            // #[debug_tree(bound(T: Debug, U: Debug))]
            if meta.path.is_ident("bound") {
                let content;
                parenthesized!(content in meta.input);
                res.bound.push(content.parse()?);
                return Ok(());
            }
            abort!(
                attr,
                "Unknown attribute. Only `desc`, `desc_field` or `desc_func` is allowed"
            );
        });
        if let Err(err) = res {
            abort!(attr, err);
        }
    }
    res
}

fn get_subtrees(fields: &Fields) -> Vec<SubTreeField> {
    let mut subtrees = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let attrs = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("debug_tree"));
        let mut is_subtree = false;
        for attr in attrs {
            let res = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("subtree") {
                    is_subtree = true;
                } else {
                    abort!(attr, "Unknown attribute. Only `subtree` is allowed");
                }
                Ok(())
            });
            if let Err(err) = res {
                abort!(attr, err);
            }
        }
        if is_subtree {
            subtrees.push(match &field.ident {
                Some(name) => SubTreeField {
                    ty: field.ty.clone(),
                    name: quote!(#name),
                },
                None => SubTreeField {
                    ty: field.ty.clone(),
                    name: syn::Index::from(i).into_token_stream(),
                },
            })
        }
    }
    subtrees
}

enum Description {
    Lit(syn::LitStr),
    Field(syn::Ident),
    Func(syn::ExprPath),
}

#[derive(Default)]
struct RootAttribs {
    use_from_qrs_datasrc: bool,
    description: Option<Description>,
    bound: Vec<syn::WherePredicate>,
}

struct SubTreeField {
    ty: Type,
    name: proc_macro2::TokenStream,
}
