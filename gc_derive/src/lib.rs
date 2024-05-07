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
    let generics = add_trait_bounds(input.generics);
    let has_generics = !generics.params.is_empty();
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
                    gc::Trace::trace(&self.#field_name, tracer);
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
                            let ident = Ident::new(&format!("t{}", idx), Span::call_site());
                            quote! {
                                gc::Trace::trace( #ident , tracer);
                            }
                        });

                        let args = fields.unnamed.iter().enumerate().map(|(idx, _)| {
                            let ident = Ident::new(&format!("t{}", idx), Span::call_site());

                            quote! { #ident }
                        });

                        quote! {
                            #name::#variant_ident(#(#args)*) => { #(#body)* }
                        }
                    }
                    Fields::Named(_) => {
                        quote! {}
                    }
                    Fields::Unit => {
                        quote! {
                            #name::#variant_ident => {}
                        }
                    }
                }
            });

            vec![quote! {
                match self { #(#arms)* }
            }]
        }
        _ => todo!("implement Derive(Trace) for union types"),
    };

    // This assert still applies to types with generics, b/c
    // the generics types are bound by the Trace trait. So for any generic trace type,
    // eventually there must be some concrete Trace type being passed in with the static,
    // assert of
    let expanded = quote! {
        unsafe impl #impl_generics gc::Trace for #name #ty_generics #where_clause {
            fn trace<GC_DERIVE_INTERNAL_TRACER_TYPE: gc::Tracer>(&self, tracer: &mut GC_DERIVE_INTERNAL_TRACER_TYPE) {
                #(#trace_body)*
            }
        }
    };

    TokenStream::from(expanded)
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Trace));
        }
    }
    generics
}
