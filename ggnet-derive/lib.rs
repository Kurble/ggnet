#![recursion_limit="128"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate byteorder;

use proc_macro::TokenStream;
use proc_macro2::Span;
use std::collections::HashSet;
use syn::*;

fn impl_reflect_struct(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let mut field_id = Vec::<Member>::new();
    let mut field_ty = HashSet::new();

    match &ast.data {
        &Data::Struct(ref data) => {
            match &data.fields {
                &Fields::Named(ref fields) => {
                    for i in fields.named.iter() {
                        field_id.push(Member::Named(i.ident.clone().unwrap()));
                        field_ty.insert(i.ty.clone());
                    }
                },
                &Fields::Unnamed(ref fields) => {
                    for (i, f) in fields.unnamed.iter().enumerate() {
                        field_id.push(Member::Unnamed(Index::from(i)));
                        field_ty.insert(f.ty.clone());
                    }
                },
                &Fields::Unit => {
                    /* no fields */
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

    let field_str: Vec<Member> = field_id.clone();

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
    let mut field_ty = HashSet::new();

    let (variants, indices, reflect_variants, construct_variants) = match &ast.data {
        &Data::Enum(ref data) => {
            let mut variants = Vec::new();
            let mut indices = Vec::new();
            let mut reflect_variants = Vec::new();
            let mut construct_variants = Vec::new();

            let mut index = 0;
            for v in data.variants.iter() {
                let ident = &v.ident;

                indices.push(index);

                match &v.fields {
                    &Fields::Unit => {
                        variants.push(quote!{#name::#ident});
                        reflect_variants.push(quote!{#name::#ident => ()});
                        construct_variants.push(quote!{#name::#ident});
                    },

                    &Fields::Named(ref fields) => {
                        for f in fields.named.iter() {
                            field_ty.insert(f.ty.clone());
                        }

                        let types: Vec<Type> = fields.named.iter()
                            .map(|f| f.ty.clone())
                            .collect();
                        let names: Vec<Ident> = fields.named.iter()
                            .map(|f| f.ident.clone().unwrap())
                            .collect();
                        let names2 = names.clone();
                        let names3 = names.clone();
                        let names4 = names.clone();
                        let names5 = names.clone();
                        let fields = fields.named.iter();

                        variants.push(quote!{#name::#ident { .. }});

                        reflect_variants.push(quote!{
                            &mut #name::#ident { #(ref mut #names,)* } => {
                                #(visitor.visit(stringify!(#names2), #names3)?;)*
                            }
                        });
                        
                        construct_variants.push(quote!{{
                            #(let #fields = {
                                let mut x: #types = Default::default();
                                visitor.visit(stringify!(#names4), &mut x)?;
                                x
                            };)*
                            #name::#ident { #(#names5,)* }
                        }});
                    },

                    &Fields::Unnamed(ref fields) => {
                        for f in fields.unnamed.iter() {
                            field_ty.insert(f.ty.clone());
                        }

                        let types: Vec<Type> = fields.unnamed.iter()
                            .map(|f| f.ty.clone())
                            .collect();
                        let types2 = types.clone();
                        let names: Vec<Ident> = fields.unnamed.iter()
                            .enumerate()
                            .map(|(i, _)| Ident::new(&format!("v{}", i), Span::call_site()))
                            .collect();
                        let underscores: Vec<_> = fields.unnamed.iter()
                            .map(|_| quote!(_))
                            .collect();
                        let names2 = names.clone();
                        let names3 = names.clone();
                        let names4 = names.clone();

                        variants.push(quote!{#name::#ident(#(#underscores,)*)});

                        reflect_variants.push(quote!{
                            &mut #name::#ident(#(ref mut #names),*) => {
                                #(#names2.reflect(visitor)?;)*
                            }
                        });

                        construct_variants.push(quote!{{
                            #(let #names3 = {
                                let mut x: #types2 = Default::default();
                                x.reflect(visitor)?;
                                x
                            };)*
                            #name::#ident(#(#names4),*)
                        }});
                    },
                }

                index += 1;
            }

            (variants, indices, reflect_variants, construct_variants)
        },
        _ => unreachable!(),
    };

    field_ty.insert(parse_quote!(u8));

    let decode_indices: Vec<u8> = indices.clone();
    let encode_indices: Vec<u8> = indices.clone();

    let ty_name = name.to_string();

    let mut impl_generics: Generics = ast.generics.clone();
    impl_generics.params.push(parse_quote!(V: Visitor));
    impl_generics.where_clause = Some(parse_quote!(where #(#field_ty: Reflect<V>,)*));

    let (_, type_generics, _) = ast.generics.split_for_impl();
    let (impl_generics, _, where_clause) = impl_generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics Reflect<V> for #name #type_generics #where_clause {
            fn reflect(&mut self, visitor: &mut V) -> Result<(), SerializeError> {
                let mut val: u8 = match self {
                    #(&mut #variants => #encode_indices,)*
                };

                let old_val = val;
                val.reflect(visitor)?;

                if old_val == val {
                    match self {
                        #(#reflect_variants,)*
                    }
                } else {
                    *self = match val {
                        #(#decode_indices => #construct_variants,)*
                        _ => panic!(format!("invalid enum {0} for {1}", val, #ty_name)),
                    };
                }

                
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