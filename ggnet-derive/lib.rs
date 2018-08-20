#![recursion_limit="128"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate byteorder;

use proc_macro::TokenStream;
use std::collections::HashSet;
use syn::*;

fn impl_reflect_struct(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let mut field_id = Vec::new();
    let mut field_ty = HashSet::new();

    match &ast.data {
        &Data::Struct(ref data) => {
            match &data.fields {
                &Fields::Named(ref fields) => {
                    for i in fields.named.iter() {
                        field_id.push(i.ident.as_ref().unwrap());
                        field_ty.insert(i.ty.clone());
                    }
                },
                &Fields::Unnamed(_) => {
                    panic!("Reflect only implemented for structs!");
                },
                &Fields::Unit => {
                    panic!("Reflect unit type makes no sense!");
                },
            }
        },
        _ => unreachable!(),
    };

    let mut impl_generics: Generics = ast.generics.clone();
    impl_generics.params.push(parse_quote!(V: Visitor));
    impl_generics.where_clause = Some(parse_quote!(where #(#field_ty: Reflect<V>,)*));

    let (_, type_generics, _) = ast.generics.split_for_impl();
    let (impl_generics, _, where_clause) = impl_generics.split_for_impl();

    let field_str: Vec<&syn::Ident> = field_id.clone();

    let tokens = quote! {
        impl #impl_generics Reflect<V> for #name #type_generics #where_clause {
            fn reflect(&mut self, visitor: &mut V) -> Result<(), SerializeError> {
                #(visitor.visit(stringify!(#field_str), &mut self.#field_id)?;)*
                Ok(())
            }
        }        
    };

    tokens.into()
}

fn impl_reflect_enum(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (variants, indices) = match &ast.data {
        &Data::Enum(ref data) => {
            let mut rvariants = Vec::new();
            let mut rindices = Vec::new();
            let mut index = 0;
            for v in data.variants.iter() {
                match &v.fields {
                    &Fields::Unit => (),
                    _ => panic!("#[derive(Serialize)] can only serialize unit variants for enums right now"),
                }

                let ident = &v.ident;

                rvariants.push(quote!{#name::#ident});
                rindices.push(index);
                index += 1;
            }
            (rvariants, rindices)
        },
        _ => unreachable!(),
    };

    let decode_indices: Vec<u8> = indices.clone();
    let decode_variants: Vec<_> = variants.clone();

    let encode_indices: Vec<u8> = indices.clone();
    let encode_variants: Vec<_> = variants.clone();

    let ty_name = name.to_string();

    let tokens = quote! {
        impl<V: Visitor> Reflect<V> for #name where
            u8: Reflect<V>,
        {
            fn reflect(&mut self, visitor: &mut V) -> Result<(), SerializeError> {
                let mut val: u8 = match self {
                    #(&mut #encode_variants => #encode_indices,)*
                };
                val.reflect(visitor)?;
                *self = match val {
                    #(#decode_indices => #decode_variants,)*
                    _ => panic!(format!("invalid enum {0} for {1}", val, #ty_name)),
                };
                Ok(())
            }
        }
    };

    tokens.into()
}

fn impl_reflect(ast: &DeriveInput) -> TokenStream {
    match &ast.data {
        &Data::Struct(_) => {
            impl_reflect_struct(ast)
        },
        &Data::Enum(_) => {
            impl_reflect_enum(ast)
        },
        &Data::Union(_) => {
            panic!("union not supported")
        }
    }    
}

#[proc_macro_derive(Reflect)]
pub fn refl(input: TokenStream) -> proc_macro::TokenStream {
    impl_reflect(&parse(input).unwrap())
}