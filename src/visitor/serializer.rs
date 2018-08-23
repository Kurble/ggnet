use std::hash::Hash;
use std::collections::HashMap;
use super::*;

pub struct Serializer<W: Write> {
    pub writer: W,
    pub current_node: Option<u32>,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            current_node: None,
        }
    }

    pub fn with_current_node(writer: W, current_node: u32) -> Self {
        Self {
            writer,
            current_node: Some(current_node),
        }
    }
}

impl<W: Write> Visitor for Serializer<W> {
    fn visit<T: Reflect<Serializer<W>>>(&mut self, _name: &str, val: &mut T) -> Result<(), Error> {
        Ok(val.reflect(self)?)
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl<W: Write> Reflect<Serializer<W>> for $t {
            fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), Error> {
                Ok(encode(&mut visit.writer, self)?)
            }
        }
    )
}

encodable!{ u8 }
encodable!{ i8 }
encodable!{ u16 }
encodable!{ i16 }
encodable!{ u32 }
encodable!{ i32 }
encodable!{ f32 }
encodable!{ u64 }
encodable!{ i64 }
encodable!{ f64 }
encodable!{ bool }
encodable!{ String }

impl<W, T> Reflect<Serializer<W>> for Vec<T> where 
    W: Write,
    T: Reflect<Serializer<W>>,
{
    fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), Error> {
        (self.len() as u32).reflect(visit)?;
        for e in self.iter_mut() {
            e.reflect(visit)?;
        }
        Ok(())
    }
}

impl<W, K, V> Reflect<Serializer<W>> for HashMap<K, V> where
    W: Write,
    K: Reflect<Serializer<W>> + Eq + Hash + Clone,
    V: Reflect<Serializer<W>>,
{
    fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), Error> {
        (self.len() as u32).reflect(visit)?;
        for (k, v) in self.iter_mut() {
            k.clone().reflect(visit)?;
            v.reflect(visit)?;
        }
        Ok(())
    }
}
