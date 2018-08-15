use std::io::Cursor;
use std::collections::HashMap;
use std::any::Any;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Reflect)]
pub enum UpdateOp {
    Replace,
    Update,
    VecPush,
    VecInsert,
    VecRemove,
    VecClear,
    MapInsert,
    MapRemove,
    MapClear,
}

impl Default for UpdateOp {
    fn default() -> Self {
        UpdateOp::Replace
    }
}

pub struct Updater<V> {
    ser: V,
    tag: String,
    op: UpdateOp,
    found: u32,
    nest: u32,
    key: Option<Box<Any>>,
    val: Option<Box<Any>>,
}

impl<U: Visitor> Updater<U> {
    pub fn new_update(ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::Update,
            found: 0,
            nest: 0,
            key: None,
            val: None,
        }
    }
    pub fn new_vec_push<V: Any>(ser: U, tag: String, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecPush,
            found: 0,
            nest: 0,
            key: None,
            val: Some(Box::new(val)),
        }
    }
    pub fn new_vec_insert<V: Any>(ser: U, tag: String, index: u32, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecInsert,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: Some(Box::new(val)),
        }
    }
    pub fn new_vec_remove(ser: U, tag: String, index: u32) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecRemove,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: None,
        }
    }
    pub fn new_vec_clear(ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecClear,
            found: 0,
            nest: 0,
            key: None,
            val: None,
        }
    }
    pub fn new_map_insert<K: Any, V: Any>(ser: U, tag: String, key: K, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapInsert,
            found: 0,
            nest: 0,
            key: Some(Box::new(key)),
            val: Some(Box::new(val)),
        }
    }
    pub fn new_map_remove<K: Any>(ser: U, tag: String, index: K) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapRemove,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: None,
        }
    }
    pub fn new_map_clear(ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapClear,
            found: 0,
            nest: 0,
            key: None,
            val: None,
        }
    }
    pub fn unwrap(self) -> U {
        self.ser
    }
}

pub trait NodeServerExt: 
    NodeBase<TagServer> + 
    Reflect<Serializer<Vec<u8>>> +
    Reflect<Updater<Serializer<Vec<u8>>>>
{
    fn resync(&mut self) {
        let mut op = UpdateOp::Replace;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        self.reflect(&mut ser);
        self.send(ser);
    }

    fn member_modified(&mut self, mut tag: String) {
        let mut op = UpdateOp::Update;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_update(ser, tag);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_vec_push<T>(&mut self, mut tag: String, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecPush;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_vec_push(ser, tag, val);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_vec_insert<T>(&mut self, mut tag: String, index: usize, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecInsert;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_vec_insert(ser, tag, index as u32, val);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_vec_remove(&mut self, mut tag: String, index: usize) {
        let mut op = UpdateOp::VecRemove;
        let mut ser = BufferSerializer { writer: Vec::new() };
    
        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_vec_remove(ser, tag, index as u32);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_vec_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::VecClear;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_vec_clear(ser, tag);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_map_insert<K, V>(&mut self, mut tag: String, key: K, val: V) where 
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any,
        V: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::MapInsert;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_map_insert(ser, tag, key, val);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_map_remove<K>(&mut self, mut tag: String, key: K) where
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any
    {
        let mut op = UpdateOp::MapRemove;
        let mut ser = BufferSerializer { writer: Vec::new() };
        
        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_map_remove(ser, tag, key);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }

    fn member_map_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::MapClear;
        let mut ser = BufferSerializer { writer: Vec::new() };

        op.reflect(&mut ser);
        tag.reflect(&mut ser);
        let mut updater = Updater::new_map_clear(ser, tag);
        self.reflect(&mut updater);
        self.send(updater.unwrap());
    }
}

impl<T> NodeServerExt for T where
    T: NodeBase<TagServer> + Reflect<Serializer<Vec<u8>>> + Reflect<Updater<Serializer<Vec<u8>>>>,
{ 
}

pub trait CallUpdate {
    fn call_upd(&mut self, msg: Deserializer<Cursor<Vec<u8>>, TagClient>);
}

impl<T> CallUpdate for T where
    T: Reflect<Deserializer<Cursor<Vec<u8>>, TagClient>> + 
       Reflect<Updater<Deserializer<Cursor<Vec<u8>>, TagClient>>>
{
    fn call_upd(&mut self, mut msg: Deserializer<Cursor<Vec<u8>>, TagClient>) {
        let mut op = UpdateOp::default();
        op.reflect(&mut msg);
        match op {
            UpdateOp::Replace => {
                self.reflect(&mut msg);
            },
            other => {
                let mut tag = String::default();
                tag.reflect(&mut msg);

                let mut upd = Updater {
                    ser: msg,
                    tag,
                    op: other,
                    found: 0,
                    nest: 0,
                    key: None,
                    val: None,
                };

                self.reflect(&mut upd);
            },
        }
    }
}

impl<V: Visitor> Visitor for Updater<V> {
    fn visit<T: Reflect<Updater<V>>>(&mut self, name: &str, val: &mut T) {
        if self.nest > 0 {
            val.reflect(self);
        } else if name == self.tag {
            if self.found > 0 {
                panic!("duplicate tag found");
            } else {
                self.found += 1;
                self.nest += 1;

                val.reflect(self);

                self.nest -= 1;
            }
        }
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl<V: Visitor> Reflect<Updater<V>> for $t where $t: Reflect<V> {
            fn reflect(&mut self, visit: &mut Updater<V>) {
                assert!(visit.op == UpdateOp::Update);
                self.reflect(&mut visit.ser);
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

fn acquire<T: Any + Default>(val: Option<Box<Any>>) -> T {
    val.and_then(|v| v.downcast().ok().and_then(|b| *b)).unwrap_or(T::default())
}

impl<V, T> Reflect<Updater<V>> for Vec<T> where 
    V: Visitor,
    T: Reflect<V> + Any + 'static,
    Vec<T>: Reflect<V>,
    u32: Reflect<V>,
{
    fn reflect(&mut self, visit: &mut Updater<V>) {
        match visit.op {
            UpdateOp::Update => {
                self.reflect(&mut visit.ser);
            },
            UpdateOp::VecPush => {
                let mut value: T = acquire(visit.val.take());
                
                value.reflect(&mut visit.ser);

                self.push(value);
            },
            UpdateOp::VecInsert => {
                let mut index: u32 = acquire(visit.key.take());
                let mut value: T = acquire(visit.val.take());

                index.reflect(&mut visit.ser);
                value.reflect(&mut visit.ser);

                self.insert(index as usize, value);
            },
            UpdateOp::VecRemove => {
                let mut index: u32 = acquire(visit.key.take());
                
                index.reflect(&mut visit.ser);
                
                self.remove(index as usize);
            },
            UpdateOp::VecClear => {
                self.clear();
            },
            _ => {
                unimplemented!();
            },
        }
    }
}


impl<U, K, V> Reflect<Updater<U>> for HashMap<K, V> where
    U: Visitor,
    K: Reflect<U> + Any + Eq + Hash + Clone + 'static,
    V: Reflect<U> + Any + 'static,
    HashMap<K, V>: Reflect<U>,
{
    fn reflect(&mut self, visit: &mut Updater<U>) {
        match visit.op {
            UpdateOp::Update => {
                self.reflect(&mut visit.ser);
            },
            UpdateOp::MapInsert => {
                let mut index: K = acquire(visit.key.take());
                let mut value: V = acquire(visit.val.take());

                index.reflect(&mut visit.ser);
                value.reflect(&mut visit.ser);

                self.insert(index, value);
            },
            UpdateOp::MapRemove => {
                let mut index: K = acquire(visit.key.take());
                
                index.reflect(&mut visit.ser);
                
                self.remove(&index);
            },
            UpdateOp::MapClear => {
                self.clear();
            },
            _ => {
                unimplemented!();
            },
        }
    }
}
