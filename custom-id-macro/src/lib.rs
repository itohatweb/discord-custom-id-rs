use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod bittable;
mod custom_id;

#[proc_macro_derive(Bittable, attributes(bittable))]
pub fn derive_bittable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    bittable::impl_bittable(input).into()
}

#[proc_macro_derive(CustomIdDerive)]
pub fn derive_custom_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    custom_id::impl_custom_id(input).into()
}
