use proc_macro::TokenStream;
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
        }) => vec![quote! { println!("Trace {:?}", self) }],
        Data::Enum(DataEnum { variants, .. }) => variants
            .iter()
            .map(|variant| {
                let variant_name = &variant.ident;
                quote! {
                    #name::#variant_name { .. } => {
                        println!("Trace {:?}", self);
                    }
                }
            })
            .collect::<Vec<_>>(),
        _ => todo!("implement Derive(Trace) for union types"),
    };

    let expanded = quote! {
        unsafe impl #impl_generics gc::Trace for #name #ty_generics #where_clause {
            fn trace<T: gc::Tracer>(&self, tracer: &mut T) {
                #(#trace_body)*
            }

            fn dyn_trace<T: gc::Tracer>(ptr: NonNull<()>, tracer: &mut T) {
                unsafe {
                    ptr.cast::<#name>().as_ref().trace(tracer)
                }
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
