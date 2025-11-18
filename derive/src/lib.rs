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
            .map(|(i, _)| {
                let idx = syn::Index::from(i);
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

        impl #impl_generics sandpit::__MustNotDrop for #name #ty_generics #where_clause {}
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

        impl #impl_generics sandpit::__MustNotDrop for #name #ty_generics #where_clause {}
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

#[proc_macro_derive(Tag, attributes(ptr))]
pub fn tag(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut from_usize_arms = vec![];
    let mut into_usize_arms = vec![];
    let mut is_ptr_arms = vec![];
    let mut trace_arms = vec![];
    let mut extraction_methods = vec![];
    let mut creation_methods = vec![];
    let mut pointer_types = vec![];
    let num_variants;

    match input.data {
        Data::Enum(DataEnum { variants, .. }) => {
            num_variants = variants.len();
            for (idx, variant) in variants.iter().enumerate() {
                let variant_name = variant.ident.clone();
                let tag_value = idx;

                match &variant.fields {
                    Fields::Unit => {
                        // Check for #[ptr(Type)] attribute
                        let ptr_type = variant
                            .attrs
                            .iter()
                            .find(|attr| attr.path().is_ident("ptr"))
                            .map(|attr| {
                                attr.parse_args::<syn::Type>()
                                    .expect("Expected type in #[ptr(Type)] attribute")
                            });

                        from_usize_arms.push(quote! { #tag_value => Some(Self::#variant_name), });
                        into_usize_arms.push(quote! { Self::#variant_name => #tag_value, });

                        if let Some(ptr_type) = ptr_type {
                            // This is a pointer variant
                            pointer_types.push(ptr_type.clone());
                            is_ptr_arms.push(quote! { Self::#variant_name => true, });

                            // Generate trace arm for pointer variant
                            trace_arms.push(quote! {
                                Self::#variant_name => {
                                    unsafe{
                                        let gc_ptr = tagged_ptr.as_gc::<#ptr_type>();

                                        sandpit::Trace::trace(&gc_ptr, tracer);
                                    }
                                }
                            });

                            // Generate creation method for this pointer type
                            let method_name = Ident::new(
                                &format!("from_{}", variant_name.to_string().to_lowercase()),
                                Span::mixed_site(),
                            );
                            creation_methods.push(quote! {
                                pub fn #method_name<'gc>(ptr: sandpit::Gc<'gc, #ptr_type>) -> sandpit::Tagged<'gc, #name> {
                                    unsafe { 
                                        sandpit::Tagged::from_ptr(ptr, #name::#variant_name)
                                    }
                                }
                            });

                            // Generate extraction method for this pointer type
                            let extract_method_name = Ident::new(
                                &format!("get_{}", variant_name.to_string().to_lowercase()),
                                Span::mixed_site(),
                            );
                            extraction_methods.push(quote! {
                                pub fn #extract_method_name<'gc>(tagged_ptr: sandpit::Tagged<'gc, Self>) -> Option<sandpit::Gc<'gc, #ptr_type>> {
                                    if matches!(tagged_ptr.get_tag(), #name::#variant_name) {
                                        unsafe {
                                            tagged_ptr.as_gc()
                                        }
                                    } else {
                                        None
                                    }
                                }
                            });
                        } else {
                            // This is a non-pointer variant
                            is_ptr_arms.push(quote! { Self::#variant_name => false, });

                            // Generate trace arm for non-pointer variant (no tracing needed)
                            trace_arms.push(quote! {
                                Self::#variant_name => {
                                    // Non-pointer variant, nothing to trace
                                }
                            });
                        }
                    }
                    _ => panic!("Tag can only be derived for fieldless enums"),
                }
            }
        }
        _ => panic!("Tag can only be derived for fieldless enums"),
    }

    let pointer_types: Vec<Type> = pointer_types
        .into_iter()
        .map(|mut ty| {
            elide_lifetimes(&mut ty);
            ty
        })
        .collect();

    // Calculate minimum alignment from all pointer types
    let min_alignment_calculation = if pointer_types.is_empty() {
        // No pointer types, use a reasonable default
        quote! { 1 }
    } else {
        quote! {
            {
                let mut min_align = usize::MAX;
                #(
                    let align = core::mem::align_of::<sandpit::Gc<#pointer_types>>();
                    if align < min_align { min_align = align; }
                )*
                min_align
            }
        }
    };

    let expanded = quote! {
        #[automatically_derived]
        unsafe impl #impl_generics sandpit::Tag for #name #ty_generics #where_clause {
            const VARIANTS: usize = #num_variants;
            const MIN_ALIGNMENT: usize = #min_alignment_calculation;

            fn into_usize(&self) -> usize {
                match self {
                    #(#into_usize_arms)*
                }
            }

            fn from_usize(value: usize) -> Option<Self> {
                match value {
                    #(#from_usize_arms)*
                    _ => None,
                }
            }

            fn is_ptr(&self) -> bool {
                match self {
                    #(#is_ptr_arms)*
                }
            }

            fn trace_tagged<'gc>(tagged_ptr: &sandpit::Tagged<'gc, Self>, tracer: &mut sandpit::Tracer) {
                match tagged_ptr.get_tag() {
                    #(#trace_arms)*
                }
            }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #(#creation_methods)*

            #(#extraction_methods)*
        }
    };

    TokenStream::from(expanded)
}

use syn::{visit_mut::VisitMut, Lifetime, Type};

struct ElideLifetimes;

impl VisitMut for ElideLifetimes {
    fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
        *lt = syn::parse_quote!('_);
    }

    fn visit_type_reference_mut(&mut self, i: &mut syn::TypeReference) {
        i.lifetime = Some(syn::parse_quote!('_));

        syn::visit_mut::visit_type_reference_mut(self, i);
    }
}

fn elide_lifetimes(ty: &mut Type) {
    ElideLifetimes.visit_type_mut(ty);
}
