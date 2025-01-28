use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn impl_custom_id(input: DeriveInput) -> TokenStream {
    let ident = input.ident.clone();

    quote! {
        impl ::custom_id::CustomIdConv for #ident {
            fn to_custom_id(&self) -> Result<String, ::custom_id::CustomIdError> {
                use ::custom_id::Bittable;

                let bit_count = self.bit_count();
                // 100 utf16 characters
                // Each utf16 character is 2 bytes
                if bit_count > 100 * 8 * 2 {
                    return Err(::custom_id::CustomIdError::DataTooBig);
                }

                let mut bits = ::custom_id::__deps::bitvec::bitvec![0; bit_count];
                self.write_bits(&mut bits);

                let mut packed_bytes = Vec::with_capacity((bit_count + 15) / 16);

                for chunk in bits.chunks(16) {
                    let mut value: u16 = 0;
                    for (i, bit) in chunk.iter().enumerate() {
                        if *bit {
                            value |= 1 << i;
                        }
                    }
                    packed_bytes.push(value);
                }

                Ok(String::from_utf16_lossy(&packed_bytes))
            }

            fn from_custom_id(custom_id: String) -> Result<Self, ::custom_id::CustomIdError> {
                use ::custom_id::Bittable;

                let utf16: Vec<u16> = custom_id.encode_utf16().collect();

                let mut bits: ::custom_id::__deps::bitvec::vec::BitVec<usize, ::custom_id::__deps::bitvec::order::Lsb0> = ::custom_id::__deps::bitvec::vec::BitVec::with_capacity(utf16.len() * 16);

                for &value in utf16.iter() {
                    for i in 0..16 {
                        let bit = (value >> i) & 1;
                        bits.push(bit == 1);
                    }
                }

                let (_, result) = Self::from_bits(&bits)?;
                Ok(result)
            }
        }
    }
}
