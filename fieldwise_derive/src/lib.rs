extern crate proc_macro;

use proc_macro2::TokenStream as TokenStream2;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{self, Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{self, parse_macro_input, Data, DataStruct, DeriveInput, Fields, Token};

struct DeriveInfo {
    root_vis: syn::Visibility,
    root_type: syn::Ident,
    root_lens: syn::Ident,
    fields_type: syn::Ident,
}

fn derive_struct(info: &DeriveInfo, data: DataStruct) -> TokenStream2 {
    let DeriveInfo {
        root_vis,
        root_type,
        root_lens,
        fields_type,
    } = &info;

    let fields = match data.fields {
        Fields::Named(fields) => fields.named,
        Fields::Unnamed(fields) => fields.unnamed,
        Fields::Unit => syn::punctuated::Punctuated::new(),
    };

    let field_lenses = fields.iter().enumerate().map(|(index, field)| {
        let field_name = field.ident.clone().unwrap_or_else(|| {
            // for tuple structs, use index as field name
            syn::Ident::new(&format!("_{}", index), field.span())
        });

        let field_access = field.ident.as_ref()
            .map(|ident| ident.to_token_stream())
            .unwrap_or_else(|| {
                // for tuple structs, use index as field name
                syn::Index { index: index as u32, span: field.span() }
                    .to_token_stream()
            });

        let field_lens_name = syn::Ident::new(&format!("{}__{}", info.root_type, field_name), field_name.span());
        let field_type = field.ty.clone();

        quote! {
            #[derive(Clone)]
            #[allow(non_camel_case_types)]
            #root_vis struct #field_lens_name<B: ::fieldwise::Path>(B);

            impl<B: ::fieldwise::Path<Item = #root_type>> ::fieldwise::Path for #field_lens_name<B> {
                type Root = B::Root;
                type Item = #field_type;

                fn get<'a>(&self, root: &'a <Self::Root as ::fieldwise::Path>::Item) -> Option<&'a Self::Item> {
                    Some(&::fieldwise::Path::get(&self.0, root)?.#field_access)
                }

                fn get_mut<'a>(&self, root: &'a mut <Self::Root as ::fieldwise::Path>::Item) -> Option<&'a mut Self::Item> {
                    Some(&mut ::fieldwise::Path::get_mut(&self.0, root)?.#field_access)
                }
            }

            impl #fields_type {
                pub fn #field_name(&self) -> #field_lens_name<#root_lens> {
                    #field_lens_name(#root_lens)
                }
            }
        }
    }).collect::<TokenStream2>();

    quote! {
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        pub struct #fields_type;

        #field_lenses
    }
}

// fn derive_enum(info: &DeriveInfo, data: DataEnum) -> TokenStream2 {
//     let DeriveInfo {
//         root_vis,
//         root_type,
//         root_lens,
//         fields_type,
//     } = &info;
// }

#[proc_macro_derive(Fieldwise)]
pub fn derive_fieldwise(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let info = DeriveInfo {
        root_vis: input.vis,
        root_type: input.ident.clone(),
        root_lens: syn::Ident::new(&format!("{}__", input.ident), input.ident.span()),
        fields_type: syn::Ident::new(&format!("{}__Fields", input.ident), input.ident.span()),
    };

    let DeriveInfo {
        root_vis,
        root_type,
        root_lens,
        fields_type,
    } = &info;

    let root_decl = quote! {
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #root_vis struct #root_lens;

        impl ::fieldwise::Fieldwise for #root_type {
            type Root = #root_lens;
            type Fields = #fields_type;

            fn root() -> Self::Root {
                #root_lens
            }

            fn fieldwise() -> Self::Fields {
                #fields_type
            }
        }

        impl ::fieldwise::Path for #root_lens {
            type Root = #root_lens;
            type Item = #root_type;

            fn get<'a>(&self, root: &'a <Self::Root as ::fieldwise::Path>::Item) -> Option<&'a Self::Item> {
                Some(root)
            }

            fn get_mut<'a>(&self, root: &'a mut <Self::Root as ::fieldwise::Path>::Item) -> Option<&'a mut Self::Item> {
                Some(root)
            }
        }
    };

    let path_impl = match input.data {
        Data::Struct(data) => derive_struct(&info, data),
        Data::Enum(_) => panic!("cannot derive Fieldwise for enums"),
        Data::Union(_) => panic!("cannot derive Fieldwise for unions"),
    };

    TokenStream::from(quote! {
        #root_decl
        #path_impl
    })
}

struct Path {
    root_type: syn::TypePath,
    accessors: Vec<Accessor>,
}

impl Parse for Path {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        // take tokens for the root type until we see a '.':
        let root_type = input.step(|cursor| {
            let mut tokens = Vec::new();
            let mut cursor = *cursor;

            while let Some((tt, next)) = cursor.token_tree() {
                match tt {
                    proc_macro2::TokenTree::Punct(punct) if punct.as_char() == '.' => {
                        // don't touch cursor, we want to include .
                        break;
                    }
                    _ => {
                        tokens.push(tt);
                        cursor = next;
                    }
                }
            }

            let type_ = syn::parse2::<syn::TypePath>(tokens.into_iter().collect())?;
            return Ok((type_, cursor));
        })?;

        let mut accessors = Vec::new();

        while !input.is_empty() {
            accessors.push(input.parse::<Accessor>()?);
        }

        Ok(Path { root_type, accessors })
    }
}

enum Accessor {
    Field(syn::Ident),
}

impl Parse for Accessor {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        input.parse::<Token![.]>()
            .and_then(|_| {
                input.parse::<syn::Ident>().map(Accessor::Field)
                    .or_else(|_| {
                        input.parse::<syn::Index>()
                            .map(|index| syn::Ident::new(&format!("_{}", index.index), index.span()))
                            .map(Accessor::Field)
                    })
            })
    }
}

#[proc_macro]
pub fn path(path: TokenStream) -> TokenStream {
    let path = parse_macro_input!(path as Path);
    let root_type = path.root_type;

    let accessors = path.accessors.iter().map(|accessor| {
        match accessor {
            Accessor::Field(ident) => quote! {
                let lens = {
                    fn get_fieldwise<T: ::fieldwise::Path<Item = F>, F: ::fieldwise::Fieldwise>(_: &T) -> F::Fields {
                        F::fieldwise()
                    }

                    let fields = get_fieldwise(&lens);
                    ::fieldwise::Compose(lens, fields.#ident())
                };
            },
        }
    }).collect::<TokenStream2>();

    TokenStream::from(quote! {
        {
            let lens = <#root_type as ::fieldwise::Fieldwise>::root();

            #accessors

            lens
        }
    })
}
