// TODO: maybe could actually publicly release this. Not at this stage but eventually. Give back to open source and so.

mod bittable;

pub use bittable::Bittable;

pub use custom_id_macro::{Bittable, CustomIdDerive};

pub mod __deps {
    pub use bitvec;
}

#[derive(thiserror::Error, Debug)]
pub enum CustomIdError {
    #[error("Discord custom ids can store up to 100 utf16 characters. The type which was tried to be serialized is too big for that.")]
    DataTooBig,
    #[error("The type could not be deserialized")]
    DeserializationFailed,
}

pub trait CustomIdConv: Bittable + Sized {
    fn to_custom_id(&self) -> Result<String, CustomIdError>;
    fn from_custom_id(custom_id: String) -> Result<Self, CustomIdError>;
}
