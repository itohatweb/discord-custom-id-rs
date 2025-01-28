// TODO: currently this does not give syntax error:
// enum Foo {
//     Foo(#[bittable(0)] bool),
//     Bar {
//         #[bittable(0)]
//         bar: bool,
//     },
// }

use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned, Data, DeriveInput, Fields, FieldsNamed, FieldsUnnamed, Ident, Variant,
};

pub fn impl_bittable(input: DeriveInput) -> TokenStream {
    let ident = input.ident.clone();
    let span = input.span();

    let result: syn::Result<TokenStream> = match input.data.clone() {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => impl_bittable_named_fields(input, Some(fields)),
            Fields::Unit => impl_bittable_named_fields(input, None),
            Fields::Unnamed(fields) => impl_bittable_unnamed_fields(input, fields),
        },
        Data::Enum(data) => impl_bittable_enum(input, data.variants),
        _ => Err(syn::Error::new(
            span,
            "`Bittable` can only be derived to structs or enums",
        )),
    };

    match result {
        Ok(output) => output,
        Err(error) => error_impl(ident, error),
    }
}

fn error_impl(ident: Ident, error: syn::Error) -> TokenStream {
    let error = error.to_compile_error();

    quote! {
        #error

         impl ::custom_id::Bittable for #ident {
             fn bit_count(&self) -> usize {
                 unimplemented!()
             }

             fn write_bits(&self, bits: &mut ::custom_id::__deps::bitvec::slice::BitSlice) -> usize {
                 unimplemented!()
             }

             fn from_bits(bits: &::custom_id::__deps::bitvec::slice::BitSlice) -> Result<(usize, Self), ::custom_id::CustomIdError> {
                 unimplemented!()
             }
         }
    }
}

// TODO: named and unnamed could probably be merged to one function
fn impl_bittable_named_fields(
    input: DeriveInput,
    fields: Option<FieldsNamed>,
) -> syn::Result<TokenStream> {
    let ident = &input.ident;

    let fields = match fields {
        Some(fields) => fields.named.into_iter().collect(),
        None => Vec::new(),
    };

    for field in &fields {
        if !field.attrs.is_empty() {
            return Err(syn::Error::new(
                field.span(),
                "bittable() is not allowed in this location",
            ));
        }
    }

    let bitfield_impl_check = {
        let tys = fields.iter().map(|field| field.ty.clone());

        quote! {
            const _: () = {
                const fn assert_impl<T: ::custom_id::Bittable>() {}
                #( assert_impl::<#tys>(); )*
            };
        }
    };

    let fields_ident = fields
        .iter()
        .map(|field| field.ident.clone())
        .collect::<Vec<_>>();

    let tokens = quote! {
        #bitfield_impl_check

        impl ::custom_id::Bittable for #ident {
            fn bit_count(&self) -> usize {
                use ::custom_id::Bittable;
                // Starting with `0` to support Unit structs
                0 #( + self.#fields_ident.bit_count())*
            }

            fn write_bits(&self, bits: &mut ::custom_id::__deps::bitvec::slice::BitSlice) -> usize {
                use ::custom_id::Bittable;

                let mut total_written = 0;

                #(
                    total_written += self.#fields_ident.write_bits(&mut bits[total_written..]);
                )*;

                total_written
            }

            fn from_bits(bits: &::custom_id::__deps::bitvec::slice::BitSlice) -> Result<(usize, Self), ::custom_id::CustomIdError> {
                use ::custom_id::Bittable;

                let mut total_read = 0;

                #(
                    let (read, #fields_ident) = ::custom_id::Bittable::from_bits(&bits[total_read..])?;
                    total_read += read;
                )*;

                let result = #ident {
                    #( #fields_ident ),*
                };

                Ok((total_read, result))
            }
        }
    };

    Ok(tokens)
}

fn impl_bittable_unnamed_fields(
    input: DeriveInput,
    fields: FieldsUnnamed,
) -> syn::Result<TokenStream> {
    let ident = &input.ident;

    let fields = fields.unnamed.into_iter().collect::<Vec<_>>();

    for field in &fields {
        if !field.attrs.is_empty() {
            return Err(syn::Error::new(
                field.span(),
                "bittable() is not allowed in this location",
            ));
        }
    }

    let bitfield_impl_check = {
        let tys = fields.iter().map(|field| field.ty.clone());

        quote! {
            const _: () = {
                const fn assert_impl<T: ::custom_id::Bittable>() {}
                #( assert_impl::<#tys>(); )*
            };
        }
    };

    let fields_idx = (0..fields.len()).map(syn::Index::from).collect::<Vec<_>>();
    let fields_store_idents = (0..fields.len())
        .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
        .collect::<Vec<_>>();

    let tokens = quote! {
        #bitfield_impl_check

        impl ::custom_id::Bittable for #ident {
            fn bit_count(&self) -> usize {
                use ::custom_id::Bittable;

                // Starting with `0` to support empty unnamed structs
                0 #( + self.#fields_idx.bit_count())*
            }

            fn write_bits(&self, bits: &mut ::custom_id::__deps::bitvec::slice::BitSlice) -> usize {
                use ::custom_id::Bittable;

                let mut total_written = 0;

                #(
                    total_written += self.#fields_idx.write_bits(&mut bits[total_written..]);
                )*

                total_written
            }

            fn from_bits(bits: &::custom_id::__deps::bitvec::slice::BitSlice) -> Result<(usize, Self), ::custom_id::CustomIdError> {
                use ::custom_id::Bittable;

                let mut total_read = 0;

                #(
                    let (read, #fields_store_idents) = ::custom_id::Bittable::from_bits(&bits[total_read..])?;
                    total_read += read;
                )*;

                let result = #ident { #( #fields_idx : #fields_store_idents ),* };

                Ok((total_read, result))
            }
        }
    };

    Ok(tokens)
}

fn impl_bittable_enum(
    input: DeriveInput,
    variants: impl IntoIterator<Item = Variant>,
) -> syn::Result<TokenStream> {
    let ident = input.ident;

    let variants: Vec<_> = variants.into_iter().collect();
    let attr_used = variants.iter().any(|variant| {
        variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("bittable"))
    });

    let mut ids = Vec::with_capacity(variants.len());

    if attr_used {
        for variant in &variants {
            let Some(attr) = variant
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("bittable"))
            else {
                return Err(syn::Error::new(
                    variant.span(),
                    "If #[bittable(id)] is used, then it must be provided for all variants.",
                ));
            };

            // TODO: Very unlikely that there will be more than u16 variants
            let literal = attr.parse_args::<Literal>()?;
            let Ok(id) = literal.to_string().parse::<u16>() else {
                return Err(syn::Error::new(
                    attr.span(),
                    "The provided variant id is not an unsigned int".to_string(),
                ));
            };

            if ids.contains(&id) {
                return Err(syn::Error::new(
                    literal.span(),
                    "The ids provided in #[bittable(id)] need to be unique",
                ));
            }

            ids.push(id);
        }
    } else {
        (0..variants.len() as u16).for_each(|id| ids.push(id));
    }

    let size_match = variants.iter().map(gen_size_match_arm);

    let write_match = variants
        .iter()
        .zip(ids.iter())
        .map(|(variant, id)| gen_write_match_arm(*id, variant));

    let read_match = variants
        .iter()
        .zip(ids.iter())
        .map(|(variant, id)| gen_read_match_arm(*id, variant));

    let tokens = quote! {
        impl ::custom_id::Bittable for #ident {
            fn bit_count(&self) -> usize {
                use ::custom_id::Bittable;

                // TODO: currently all enums are serialized as a u16 number
                16 + match self {
                    #( #size_match )*
                }
            }

            fn write_bits(&self, bits: &mut ::custom_id::__deps::bitvec::slice::BitSlice) -> usize {
                use ::custom_id::Bittable;

                let mut total_written = 0;

                match self {
                    #( #write_match )*
                }

                total_written
            }

            fn from_bits(bits: &::custom_id::__deps::bitvec::slice::BitSlice) -> Result<(usize, Self), ::custom_id::CustomIdError> {
                use ::custom_id::Bittable;

                let mut total_read = 0;

                let (bits_read, id) = u16::from_bits(&bits[total_read..])?;
                total_read += bits_read;

                let result = match id {
                    #( #read_match )*
                    _ => todo!()
                };

                Ok((total_read, result))
            }
        }
    };

    Ok(tokens)
}

fn gen_size_match_arm(variant: &Variant) -> TokenStream {
    let ident = variant.ident.clone();
    let span = variant.span();

    match &variant.fields {
        Fields::Named(fields) => {
            let field = fields
                .named
                .iter()
                .map(|field| field.clone().ident)
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span => Self::#ident { }  => 0, }
            } else {
                quote_spanned! { span => Self::#ident { #( #field ),* } => #( #field.bit_count() )+* , }
            }
        }
        Fields::Unnamed(fields) => {
            let field = (0..fields.unnamed.len())
                .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span => Self::#ident ( )  => 0, }
            } else {
                quote_spanned! { span => Self::#ident( #( #field ),* ) => #( #field.bit_count() )+* , }
            }
        }
        Fields::Unit => quote_spanned! { span => Self::#ident => 0, },
    }
}

fn gen_write_match_arm(id: u16, variant: &Variant) -> TokenStream {
    let ident = variant.ident.clone();
    let span = variant.span();

    match &variant.fields {
        Fields::Named(fields) => {
            let field = fields
                .named
                .iter()
                .map(|field| field.clone().ident)
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span =>
                    Self::#ident { } => {
                        total_written += #id.write_bits(&mut bits[total_written..]);
                    }
                }
            } else {
                quote_spanned! { span =>
                    Self::#ident { #( #field ),* } => {
                        total_written += #id.write_bits(&mut bits[total_written..]);

                        #(
                            total_written += #field.write_bits(&mut bits[total_written..]);
                        )*

                    }
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field = (0..fields.unnamed.len())
                .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span =>
                    Self::#ident ( )  => {
                        total_written += #id.write_bits(&mut bits[total_written..]);
                    }
                }
            } else {
                quote_spanned! { span =>
                    Self::#ident( #( #field ),* ) => {
                        total_written += #id.write_bits(&mut bits[total_written..]);

                        #(
                            total_written += #field.write_bits(&mut bits[total_written..]);
                        )*
                    }
                }
            }
        }
        Fields::Unit => quote_spanned! { span =>
            Self::#ident => {
                total_written += #id.write_bits(&mut bits[total_written..]);
            }
        },
    }
}

fn gen_read_match_arm(id: u16, variant: &Variant) -> TokenStream {
    let ident = variant.ident.clone();
    let span = variant.span();

    match &variant.fields {
        Fields::Named(fields) => {
            let field = fields
                .named
                .iter()
                .map(|field| field.clone().ident)
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span =>
                    #id => {
                        Self::#ident { }
                    }
                }
            } else {
                quote_spanned! { span =>
                    #id => {
                        #(
                            let (read, #field) = ::custom_id::Bittable::from_bits(&bits[total_read..])?;
                            total_read += read;
                        )*

                        Self::#ident {
                            #( #field ),*
                        }
                    }
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field = (0..fields.unnamed.len())
                .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
                .collect::<Vec<_>>();

            if field.is_empty() {
                quote_spanned! { span =>
                    #id => {
                        Self::#ident ( )
                    }
                }
            } else {
                quote_spanned! { span =>
                    #id => {
                        #(
                            let (read, #field) = ::custom_id::Bittable::from_bits(&bits[total_read..])?;
                            total_read += read;
                        )*

                        Self::#ident ( #( #field ),* )
                    }
                }
            }
        }
        Fields::Unit => quote_spanned! { span =>
            #id => {
                Self::#ident
            }
        },
    }
}
