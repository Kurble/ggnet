use std;
use std::io::Read;
use std::io::Write;
use std::hash::Hash;
use std::string::String;
use std::collections::HashMap;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

pub fn decode<R: Read, T: FromStream<R>>(stream: &mut R) -> Result<T, SerializeError> {
    T::decode(stream)
}

pub fn encode<W: Write, T: ToStream<W>>(stream: &mut W, val: &T) -> Result<(), SerializeError> {
    val.encode(stream)
}

#[derive(Debug)]
pub enum SerializeError {
	IOError(std::io::Error),
    UTFError(std::string::FromUtf8Error),
}

impl From<std::io::Error> for SerializeError {
    fn from(err: std::io::Error) -> SerializeError {
        SerializeError::IOError(err)
    }
}

impl From<std::string::FromUtf8Error> for SerializeError {
    fn from(err: std::string::FromUtf8Error) -> SerializeError {
        SerializeError::UTFError(err)
    }
}

pub trait FromStream<R: Read> where Self: Sized + Clone {
    fn decode(stream: &mut R) -> Result<Self, SerializeError>;
}

pub trait ToStream<W: Write> where Self: Sized + Clone {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError>;
}

impl <R: ReadBytesExt> FromStream<R> for u8 {
    fn decode(stream: &mut R) -> Result<u8, SerializeError> {
        Ok(stream.read_u8()?)
    }
}

impl <W: WriteBytesExt> ToStream<W> for u8 {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        Ok(stream.write_u8(*self)?)
    }
}

impl <R: ReadBytesExt> FromStream<R> for i8 {
    fn decode(stream: &mut R) -> Result<i8, SerializeError> {
        Ok(stream.read_i8()?)
    }
}

impl <W: WriteBytesExt> ToStream<W> for i8 {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        Ok(stream.write_i8(*self)?)
    }
}

macro_rules! primitive {
    ($t:ty, $rdr:ident, $wrt:ident) => (
        impl<R: ReadBytesExt> FromStream<R> for $t {
            fn decode(stream: &mut R) -> Result<$t, SerializeError> {
                Ok(stream.$rdr::<BigEndian>()?)
            }
        }
        impl<W: WriteBytesExt> ToStream<W> for $t {
            fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
                Ok(stream.$wrt::<BigEndian>(*self)?)
            }
        }
    )
}

primitive!{ u16, read_u16, write_u16 }
primitive!{ i16, read_i16, write_i16 }
primitive!{ u32, read_u32, write_u32 }
primitive!{ i32, read_i32, write_i32 }
primitive!{ f32, read_f32, write_f32 }
primitive!{ u64, read_u64, write_u64 }
primitive!{ i64, read_i64, write_i64 }
primitive!{ f64, read_f64, write_f64 }

impl <R: ReadBytesExt> FromStream<R> for bool {
    fn decode(stream: &mut R) -> Result<bool, SerializeError> {
        if stream.read_u8()? > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}   

impl <W: WriteBytesExt> ToStream<W> for bool {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        Ok(stream.write_u8(if *self { 1u8 } else { 0u8 })?)
    }
}

impl <R: ReadBytesExt, T: FromStream<R>> FromStream<R> for Vec<T> {
    fn decode(stream: &mut R) -> Result<Vec<T>, SerializeError> {
        let mut result = Vec::new();
        let count: u32 = decode(stream)?;
        for _ in 0..count {
            result.push(decode(stream)?);
        }
        Ok(result)
    }
}

impl <W: WriteBytesExt, T: ToStream<W>> ToStream<W> for Vec<T> {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        encode(stream, &(self.len() as u32))?;
        for i in self.iter() {
            encode(stream, i)?;
        }
        Ok(())
    }
}

impl <'a, W: WriteBytesExt> ToStream<W> for &'a[u8] {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        encode(stream, &(self.len() as u32))?;
        for i in self.iter() {
            encode(stream, i)?;
        }
        Ok(())
    }
}

impl <R: ReadBytesExt, K: FromStream<R> + Eq + Hash, V: FromStream<R>> FromStream<R> for HashMap<K,V> {
    fn decode(stream: &mut R) -> Result<HashMap<K,V>, SerializeError> {
        let mut result = HashMap::new();
        let count: u32 = decode(stream)?;
        for _ in 0..count {
            result.insert(decode(stream)?, decode(stream)?);
        }
        Ok(result)
    }
}

impl <W: WriteBytesExt, K: ToStream<W> + Eq + Hash, V: ToStream<W>> ToStream<W> for HashMap<K,V> {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        encode(stream, &(self.len() as u32))?;
        for (k,v) in self.iter() {
            encode(stream, k)?;
            encode(stream, v)?;
        }
        Ok(())
    }
}

impl <R: ReadBytesExt> FromStream<R> for String {
    fn decode(stream: &mut R) -> Result<String, SerializeError> {
        let content = Vec::<u8>::decode(stream)?;
        Ok(String::from_utf8(content)?)
    }
}

impl <W: WriteBytesExt> ToStream<W> for String {
    fn encode(&self, stream: &mut W) -> Result<(), SerializeError> {
        self.as_bytes().encode(stream)
    }
}