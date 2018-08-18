use std::hash::Hash;
use std::collections::HashMap;
use super::*;

pub struct Deserializer<R: Read> {
    pub reader: R,
}

impl<R: Read> Deserializer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader
        }
    }
}

impl<R: Read> Visitor for Deserializer<R> {
    fn visit<T: Reflect<Deserializer<R>>>(&mut self, _name: &str, val: &mut T) -> Result<(), SerializeError> {
        Ok(val.reflect(self)?)
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl<R: Read> Reflect<Deserializer<R>> for $t {
            fn reflect(&mut self, visit: &mut Deserializer<R>) -> Result<(), SerializeError> {
                *self = decode(&mut visit.reader)?;
                Ok(())
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

impl<R, T> Reflect<Deserializer<R>> for Vec<T> where 
    R: Read,
    T: Reflect<Deserializer<R>>,
{
    fn reflect(&mut self, visit: &mut Deserializer<R>) -> Result<(), SerializeError> {
        let mut len = 0u32;
        len.reflect(visit)?;
        self.clear();
        for _ in 0..len {
            self.push(T::default());
            self.last_mut().unwrap().reflect(visit)?;
        }
        Ok(())
    }
}

impl<R, K, V> Reflect<Deserializer<R>> for HashMap<K, V> where
    R: Read,
    K: Reflect<Deserializer<R>> + Eq + Hash + Clone,
    V: Reflect<Deserializer<R>>,
{
    fn reflect(&mut self, visit: &mut Deserializer<R>) -> Result<(), SerializeError> {
        let mut len = 0u32;
        len.reflect(visit)?;
        self.clear();
        for _ in 0..len {
            let mut k = K::default();
            let mut v = V::default();

            k.reflect(visit)?;
            v.reflect(visit)?;

            self.insert(k, v);
        }
        Ok(())
    }
}
