extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate byteorder;

use proc_macro::TokenStream;
use std::collections::HashSet;

fn impl_reflect_struct(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let mut field_id = Vec::new();
    let mut field_ty = HashSet::new();

    match &ast.body {
        &syn::Body::Struct(ref variant_data) => {
            match variant_data {
                &syn::VariantData::Struct(ref fields) => {
                    for i in fields.iter() {
                        field_id.push(i.ident.as_ref().unwrap());
                        field_ty.insert(i.ty.clone());
                    }
                },
                &syn::VariantData::Tuple(_) => {
                    panic!("Reflect only implemented for structs!");
                },
                &syn::VariantData::Unit => {
                    panic!("Reflect unit type makes no sense!");
                },
            }
        },
        _ => unreachable!(),
    };

    let field_str: Vec<&syn::Ident> = field_id.clone();

    quote! {
        impl<V: Visitor> Reflect<V> for #name where
            #(#field_ty: Reflect<V>,)*
        {
            fn reflect(&mut self, visitor: &mut V) {
                #(visitor.visit(stringify!(#field_str), &mut self.#field_id);)*
            }
        }        
    }
}

fn impl_reflect_enum(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;

    let (variants, indices) = match &ast.body {
        &syn::Body::Enum(ref variants) => {
            let mut rvariants = Vec::new();
            let mut rindices = Vec::new();
            let mut index = 0;
            for v in variants.iter() {
                match &v.data {
                    &syn::VariantData::Unit => (),
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
    let decode_variants: Vec<quote::Tokens> = variants.clone();

    let encode_indices: Vec<u8> = indices.clone();
    let encode_variants: Vec<quote::Tokens> = variants.clone();

    let ty_name = name.to_string();

    quote! {
        impl<V: Visitor> Reflect<V> for #name where
            u8: Reflect<V>,
        {
            fn reflect(&mut self, visitor: &mut V) {
                let mut val: u8 = match self {
                    #(&mut #encode_variants => #encode_indices,)*
                };
                val.reflect(visitor);
                *self = match val {
                    #(#decode_indices => #decode_variants,)*
                    _ => panic!(format!("invalid enum {0} for {1}", val, #ty_name)),
                };
            }
        }
    }
}

fn impl_reflect(ast: &syn::DeriveInput) -> quote::Tokens {
    match &ast.body {
        &syn::Body::Struct(_) => {
            impl_reflect_struct(ast)
        },
        &syn::Body::Enum(_) => {
            impl_reflect_enum(ast)
        },
    }    
}

#[proc_macro_derive(Reflect)]
pub fn refl(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_reflect(&ast);
    gen.parse().unwrap()
}