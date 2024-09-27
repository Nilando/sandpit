use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DataStruct, DeriveInput, Fields, GenericParam,
    Generics,
};

#[proc_macro_derive(Trace)]
pub fn trace(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = add_trace(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let trace_body = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(ref fields),
            ..
        }) => fields
            .named
            .iter()
            .map(|field| {
                let field_name = &field.ident;

                quote! {
                    sandpit::Trace::trace(&self.#field_name, tracer);
                }
            })
            .collect::<Vec<_>>(),
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(ref fields),
            ..
        }) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                quote! {
                    sandpit::Trace::trace(&self.#idx, tracer);
                }
            })
            .collect::<Vec<_>>(),
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => vec![quote! {}],
        Data::Enum(DataEnum { variants, .. }) => {
            let arms = variants.iter().map(|variant| {
                let variant_ident = &variant.ident;

                match &variant.fields {
                    Fields::Unnamed(fields) => {
                        let body = fields.unnamed.iter().enumerate().map(|(idx, _)| {
                            let ident = Ident::new(&format!("t{}", idx), Span::mixed_site());
                            quote! {
                                sandpit::Trace::trace( #ident , tracer);
                            }
                        });

                        let args = fields.unnamed.iter().enumerate().map(|(idx, _)| {
                            let ident = Ident::new(&format!("t{}", idx), Span::mixed_site());

                            quote! { #ident, }
                        });

                        quote! {
                            #name::#variant_ident(#(#args)*) => { #(#body)* }
                        }
                    }
                    Fields::Named(fields) => {
                        let body = fields.named.iter().map(|field| {
                            let ident = field.ident.clone().unwrap();

                            quote! {
                                sandpit::Trace::trace( #ident , tracer);
                            }
                        });

                        let args = fields.named.iter().map(|field| {
                            let ident = field.ident.clone().unwrap();

                            quote! { #ident, }
                        });

                        quote! {
                            #name::#variant_ident{#(#args)*} => { #(#body)* }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            #name::#variant_ident => {}
                        }
                    }
                }
            });

            if variants.is_empty() {
                vec![quote! {}]
            } else {
                vec![quote! {
                    match self { #(#arms)* }
                }]
            }
        }
        _ => unimplemented!("#[derive(Trace)] is not implemented for this type"),
    };

    // This assert still applies to types with generics, b/c
    // the generics types are bound by the Trace trait. So for any generic trace type,
    // eventually there must be some concrete Trace type being passed in with the static,
    // assert of
    let expanded = quote! {
        #[automatically_derived]
        unsafe impl #impl_generics sandpit::Trace for #name #ty_generics #where_clause {
            const IS_LEAF: bool = false;

            fn trace(&self, tracer: &mut sandpit::Tracer) {
                #(#trace_body)*
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(TraceLeaf)]
pub fn traceleaf(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = add_leaf(add_trace(input.generics));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let trace_body = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(ref fields),
            ..
        }) => fields
            .named
            .iter()
            .map(|field| {
                let ty = &field.ty;

                quote! {
                    <#ty as TraceLeaf>::__assert_trace_leaf();
                }
            })
            .collect::<Vec<_>>(),
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => vec![quote! {}],
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(ref fields),
            ..
        }) => fields
            .unnamed
            .iter()
            .map(|field| {
                let ty = &field.ty;

                quote! {
                    <#ty as TraceLeaf>::__assert_trace_leaf();
                }
            })
            .collect::<Vec<_>>(),
        Data::Enum(DataEnum { variants, .. }) => {
            let arms = variants
                .iter()
                .map(|variant| match &variant.fields {
                    Fields::Unnamed(fields) => fields
                        .unnamed
                        .iter()
                        .map(|field| {
                            let ty = &field.ty;

                            quote! {
                                <#ty as TraceLeaf>::__assert_trace_leaf();
                            }
                        })
                        .collect::<Vec<_>>(),
                    Fields::Named(fields) => fields
                        .named
                        .iter()
                        .map(|field| {
                            let ty = &field.ty;

                            quote! {
                                <#ty as TraceLeaf>::__assert_trace_leaf();
                            }
                        })
                        .collect::<Vec<_>>(),
                    Fields::Unit => vec![quote! {}],
                })
                .collect::<Vec<_>>();

            arms.into_iter().flatten().collect()
        }
        _ => unimplemented!("#[derive(TraceLeaf)] is not implemented for this type"),
    };

    // This assert still applies to types with generics, b/c
    // the generics types are bound by the Trace trait. So for any generic trace type,
    // eventually there must be some concrete Trace type being passed in with the static,
    // assert of
    let expanded = quote! {
        #[automatically_derived]
        unsafe impl #impl_generics sandpit::TraceLeaf for #name #ty_generics #where_clause {
            fn __assert_trace_leaf() {
                #(#trace_body)*
            }
        }
        unsafe impl #impl_generics sandpit::Trace for #name #ty_generics #where_clause {
            const IS_LEAF: bool = false;

            fn trace(&self, tracer: &mut sandpit::Tracer) {}
        }
    };

    TokenStream::from(expanded)
}

fn add_trace(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(sandpit::Trace));
        }
    }
    generics
}

fn add_leaf(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(sandpit::TraceLeaf));
        }
    }
    generics
}
