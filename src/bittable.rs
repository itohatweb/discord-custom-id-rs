// Probably shouldn't show this to anyone who studied CS that's a nightmare implementation

// TODO: decide whether a panic is ok for `Bittable::write_bits` should be tbh

// TODO: Probably it would be worth to add some unit tests for that lol

// TODO: improve error handling like things should not overflow and also it should not return custom ids longer than 100 utf16 characters (dont use .len() but .chars().len()).

use bitvec::prelude::*;

use crate::CustomIdError;

// TODO: Bittable is actually not that good of a name for this. But it's funny so ima change it later
pub trait Bittable: Sized {
    fn bit_count(&self) -> usize;
    fn write_bits(&self, bits: &mut BitSlice) -> usize;
    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError>;
}

impl Bittable for bool {
    fn bit_count(&self) -> usize {
        1
    }

    fn write_bits(&self, dest: &mut BitSlice) -> usize {
        assert!(
            !dest.is_empty(),
            "BitSlice should be created with the correct size before writing"
        );

        dest.set(0, *self);

        1
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let res = bits.get(0).map_or(false, |b| *b);

        Ok((1, res))
    }
}

macro_rules! impl_bittable_for_int {
    ($( $type:ty ),+) => {
        $(
            impl Bittable for $type {
                fn bit_count(&self) -> usize {
                    Self::BITS as usize
                }

                fn write_bits(&self, dest: &mut BitSlice) -> usize {
                    assert!(
                        dest.len() >= Self::BITS as usize,
                        "BitSlice should be created with the correct size before writing"
                    );

                    let num = *self;

                    for i in (0..Self::BITS as usize).rev() {
                        dest.set(i, (num >> i) & 1 == 1);
                    }

                    Self::BITS as usize
                }

                fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
                    if bits.len() < Self::BITS as usize {
                        return Err(CustomIdError::DeserializationFailed);
                    }

                    let mut result: Self = 0;

                    for i in 0..(Self::BITS as usize) {
                        if bits.get(i).map_or(false, |b| *b) {
                            result |= 1 << (i);
                        }
                    }

                    Ok((Self::BITS as usize, result))
                }
            }
        )+
    };
}

impl_bittable_for_int!(u8, i8, u16, i16, u32, i32, u64, i64, i128, u128, usize, isize);

impl Bittable for f32 {
    fn bit_count(&self) -> usize {
        u32::BITS as usize
    }

    fn write_bits(&self, dest: &mut BitSlice) -> usize {
        self.to_bits().write_bits(dest)
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (read, raw_bits) = u32::from_bits(bits)?;

        Ok((read, f32::from_bits(raw_bits)))
    }
}

impl Bittable for f64 {
    fn bit_count(&self) -> usize {
        u64::BITS as usize
    }

    fn write_bits(&self, dest: &mut BitSlice) -> usize {
        self.to_bits().write_bits(dest)
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (read, raw_bits) = u64::from_bits(bits)?;

        Ok((read, f64::from_bits(raw_bits)))
    }
}

impl Bittable for String {
    fn bit_count(&self) -> usize {
        // `u8::BITS +` because the length of the string is stored as a `u8`
        // `u8` because the utf16 custom id length limit is `100` => `200` utf8 characters can be stored
        u8::BITS as usize + (self.as_bytes().len() * u8::BITS as usize)
    }

    fn write_bits(&self, dest: &mut BitSlice) -> usize {
        assert!(
            dest.len() >= self.bit_count(),
            "BitSlice should be created with the correct size before writing"
        );

        // TODO: overflow check
        let length = self.as_bytes().len() as u8;

        let mut total_written = 0;
        total_written += length.write_bits(dest);
        for byte in self.as_bytes() {
            total_written += byte.write_bits(&mut dest[total_written..]);
        }

        self.bit_count()
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (mut total_read, length) = u8::from_bits(bits)?;

        let mut bytes = Vec::with_capacity(length as usize);

        for _ in 0..length as usize {
            let (read, byte) = u8::from_bits(&bits[total_read..])?;
            total_read += read;
            bytes.push(byte);
        }

        let string = match String::from_utf8(bytes) {
            Ok(string) => string,
            Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        };

        Ok((total_read, string))
    }
}

impl<T: Bittable> Bittable for Vec<T> {
    fn bit_count(&self) -> usize {
        // `u8::BITS +` because the length of the string is stored as a `u8`
        // `u8` because the utf16 custom id length limit is `100`.
        u8::BITS as usize + self.iter().fold(0, |acc, item| acc + item.bit_count())
    }

    fn write_bits(&self, dest: &mut BitSlice) -> usize {
        assert!(
            dest.len() >= self.bit_count(),
            "BitSlice should be created with the correct size before writing"
        );

        // TODO: overflow check
        let length = self.len() as u8;

        let mut total_written = 0;
        total_written += length.write_bits(dest);
        for item in self {
            total_written += item.write_bits(&mut dest[total_written..]);
        }

        self.bit_count()
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (mut total_read, length) = u8::from_bits(bits)?;

        let mut result = Vec::with_capacity(length as usize);

        for _ in 0..length as usize {
            let (read, byte) = <T>::from_bits(&bits[total_read..])?;
            total_read += read;
            result.push(byte);
        }

        Ok((total_read, result))
    }
}

impl<T> Bittable for twilight_model::id::Id<T> {
    fn bit_count(&self) -> usize {
        u64::BITS as usize
    }

    fn write_bits(&self, bits: &mut BitSlice) -> usize {
        self.get().write_bits(bits)
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (read, id) = u64::from_bits(bits)?;

        Ok((read, Self::new(id)))
    }
}

impl<T: Bittable> Bittable for Option<T> {
    fn bit_count(&self) -> usize {
        1 + match self {
            Some(item) => item.bit_count(),
            None => 0,
        }
    }

    fn write_bits(&self, bits: &mut BitSlice) -> usize {
        let mut total_written = self.is_some().write_bits(bits);

        if let Some(item) = self {
            total_written += item.write_bits(&mut bits[total_written..]);
        }

        total_written
    }

    fn from_bits(bits: &BitSlice) -> Result<(usize, Self), CustomIdError> {
        let (mut total_read, exists) = bool::from_bits(bits)?;

        if !exists {
            return Ok((total_read, None));
        }

        let (read, item) = <T>::from_bits(&bits[total_read..])?;
        total_read += read;

        Ok((total_read, Some(item)))
    }
}
