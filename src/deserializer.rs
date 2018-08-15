use std::hash::Hash;
use std::collections::HashMap;
use super::*;

pub struct Deserializer<R: Read, T: Tag> {
    pub reader: R,
    pub context: Box<NodeContext<T>>,
}

impl<R: Read, G: Tag> Visitor for Deserializer<R, G> {
    fn visit<T: Reflect<Deserializer<R, G>>>(&mut self, _name: &str, val: &mut T) -> Result<(), SerializeError> {
        Ok(val.reflect(self)?)
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl<R: Read, T: Tag> Reflect<Deserializer<R, T>> for $t {
            fn reflect(&mut self, visit: &mut Deserializer<R, T>) -> Result<(), SerializeError> {
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

impl<R, T, G> Reflect<Deserializer<R, G>> for Vec<T> where 
    R: Read,
    T: Reflect<Deserializer<R, G>>,
    G: Tag,
{
    fn reflect(&mut self, visit: &mut Deserializer<R, G>) -> Result<(), SerializeError> {
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

impl<R, T, K, V> Reflect<Deserializer<R, T>> for HashMap<K, V> where
    R: Read,
    T: Tag,
    K: Reflect<Deserializer<R, T>> + Eq + Hash + Clone,
    V: Reflect<Deserializer<R, T>>,
{
    fn reflect(&mut self, visit: &mut Deserializer<R, T>) -> Result<(), SerializeError> {
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
