use super::*;
use std::io::Cursor;
use std::collections::HashMap;
use std::any::Any;
use node::{BufferSerializer, BufferDeserializer, NodeBase};

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

    pub current_node: u32,
}

impl<U: Visitor> Updater<U> {
    pub fn new_replace(node: u32, ser: U) -> Self {
        Self {
            ser,
            tag: String::default(),
            op: UpdateOp::Replace,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: node,
        }
    }
    pub fn new_update(node: u32, ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::Update,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: node,
        }
    }
    pub fn new_vec_push<V: Any>(node: u32, ser: U, tag: String, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecPush,
            found: 0,
            nest: 0,
            key: None,
            val: Some(Box::new(val)),
            current_node: node,
        }
    }
    pub fn new_vec_insert<V: Any>(node: u32, ser: U, tag: String, index: u32, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecInsert,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: Some(Box::new(val)),
            current_node: node,
        }
    }
    pub fn new_vec_remove(node: u32, ser: U, tag: String, index: u32) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecRemove,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: None,
            current_node: node,
        }
    }
    pub fn new_vec_clear(node: u32, ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::VecClear,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: node,
        }
    }
    pub fn new_map_insert<K: Any, V: Any>(node: u32, ser: U, tag: String, key: K, val: V) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapInsert,
            found: 0,
            nest: 0,
            key: Some(Box::new(key)),
            val: Some(Box::new(val)),
            current_node: node,
        }
    }
    pub fn new_map_remove<K: Any>(node: u32, ser: U, tag: String, index: K) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapRemove,
            found: 0,
            nest: 0,
            key: Some(Box::new(index)),
            val: None,
            current_node: node,
        }
    }
    pub fn new_map_clear(node: u32, ser: U, tag: String) -> Self {
        Self {
            ser,
            tag,
            op: UpdateOp::MapClear,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: node,
        }
    }
    pub fn unwrap(self) -> U {
        self.ser
    }
}

/// This trait exposes functions that can be used to notify clients op updates to the data model of
///  a `Node`. The update messages generated will be sent to all connections that hold a reference
///  to the `Node`.
pub trait NodeServerExt: 
    NodeBase<TagServer> + 
    Reflect<Serializer<Vec<u8>>> +
    Reflect<Updater<Serializer<Vec<u8>>>>
{
    /// Update all members.
    fn resync(&mut self) {
        let mut op = UpdateOp::Replace;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        String::default().reflect(&mut ser).unwrap();
        let mut updater = Updater::new_replace(self.id(), ser);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Update a single member with name `tag`.
    fn member_modified(&mut self, mut tag: String) {
        let mut op = UpdateOp::Update;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_update(self.id(), ser, tag);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Push a new element to the `Vec<T>` member with name `tag`.
    fn member_vec_push<T>(&mut self, mut tag: String, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecPush;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_push(self.id(), ser, tag, val);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Insert a new element to the `Vec<T>` member with name `tag` at position `index`.
    fn member_vec_insert<T>(&mut self, mut tag: String, index: usize, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecInsert;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_insert(self.id(), ser, tag, index as u32, val);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Remove an element from the `Vec<T>` member with name `tag` at position `index`.
    fn member_vec_remove(&mut self, mut tag: String, index: usize) {
        let mut op = UpdateOp::VecRemove;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());
    
        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_remove(self.id(), ser, tag, index as u32);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Clear the `Vec<T>` member with name `tag`.
    fn member_vec_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::VecClear;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_clear(self.id(), ser, tag);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Insert a new element to the `HashMap<K,V>` member with name `tag`.
    fn member_map_insert<K, V>(&mut self, mut tag: String, key: K, val: V) where 
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any,
        V: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::MapInsert;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_insert(self.id(), ser, tag, key, val);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Remove an element from the `Vec<T>` member with name `tag` with key `key`.
    fn member_map_remove<K>(&mut self, mut tag: String, key: K) where
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any
    {
        let mut op = UpdateOp::MapRemove;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());
        
        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_remove(self.id(), ser, tag, key);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Clear the `HashMap<K,V>` member with name `tag`.
    fn member_map_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::MapClear;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_clear(self.id(), ser, tag);
        self.reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }
}

impl<T> NodeServerExt for T where
    T: NodeBase<TagServer> + Reflect<Serializer<Vec<u8>>> + Reflect<Updater<Serializer<Vec<u8>>>>,
{ 
}

pub trait CallUpdate {
    fn call_upd(&mut self, msg: BufferDeserializer);
}

impl<T> CallUpdate for T where
    T: Reflect<Deserializer<Cursor<Vec<u8>>>> + 
       Reflect<Updater<Deserializer<Cursor<Vec<u8>>>>>
{
    fn call_upd(&mut self, mut msg: BufferDeserializer) {
        let mut op = UpdateOp::default();
        let mut tag = String::default();

        op.reflect(&mut msg).unwrap();
        tag.reflect(&mut msg).unwrap();

        let mut upd = Updater {
            ser: msg,
            tag,
            op,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: 0,
        };

        self.reflect(&mut upd).unwrap();
    }
}

impl<V: Visitor> Visitor for Updater<V> {
    fn visit<T: Reflect<Updater<V>>>(&mut self, name: &str, val: &mut T) -> Result<(), Error> {
        if self.op == UpdateOp::Replace {
            val.reflect(self)?;
        } else if self.nest > 0 {
            val.reflect(self)?;
        } else if name == self.tag {
            if self.found > 0 {
                panic!("duplicate tag found");
            } else {
                self.found += 1;
                self.nest += 1;

                val.reflect(self)?;

                self.nest -= 1;
            }
        }
        Ok(())
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl<V: Visitor> Reflect<Updater<V>> for $t where $t: Reflect<V> {
            fn reflect(&mut self, visit: &mut Updater<V>) -> Result<(), Error> {
                assert!(visit.op == UpdateOp::Update || visit.op == UpdateOp::Replace);
                Ok(self.reflect(&mut visit.ser)?)
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
    val.and_then(|v| Some(*v.downcast().unwrap())).unwrap_or(T::default())
}

impl<V, T> Reflect<Updater<V>> for Vec<T> where 
    V: Visitor,
    T: Reflect<V> + Any + 'static,
    Vec<T>: Reflect<V>,
    u32: Reflect<V>,
{
    fn reflect(&mut self, visit: &mut Updater<V>) -> Result<(), Error> {
        match visit.op {
            UpdateOp::Update | UpdateOp::Replace => {
                self.reflect(&mut visit.ser)?;
            },
            UpdateOp::VecPush => {
                let mut value: T = acquire(visit.val.take());
                
                value.reflect(&mut visit.ser)?;

                self.push(value);
            },
            UpdateOp::VecInsert => {
                let mut index: u32 = acquire(visit.key.take());
                let mut value: T = acquire(visit.val.take());

                index.reflect(&mut visit.ser)?;
                value.reflect(&mut visit.ser)?;

                self.insert(index as usize, value);
            },
            UpdateOp::VecRemove => {
                let mut index: u32 = acquire(visit.key.take());
                
                index.reflect(&mut visit.ser)?;
                
                self.remove(index as usize);
            },
            UpdateOp::VecClear => {
                self.clear();
            },
            _ => {
                unimplemented!();
            },
        }
        Ok(())
    }
}


impl<U, K, V> Reflect<Updater<U>> for HashMap<K, V> where
    U: Visitor,
    K: Reflect<U> + Any + Eq + Hash + Clone + 'static,
    V: Reflect<U> + Any + 'static,
    HashMap<K, V>: Reflect<U>,
{
    fn reflect(&mut self, visit: &mut Updater<U>) -> Result<(), Error> {
        match visit.op {
            UpdateOp::Update | UpdateOp::Replace => {
                self.reflect(&mut visit.ser)?;
            },
            UpdateOp::MapInsert => {
                let mut index: K = acquire(visit.key.take());
                let mut value: V = acquire(visit.val.take());

                index.reflect(&mut visit.ser)?;
                value.reflect(&mut visit.ser)?;

                self.insert(index, value);
            },
            UpdateOp::MapRemove => {
                let mut index: K = acquire(visit.key.take());
                
                index.reflect(&mut visit.ser)?;
                
                self.remove(&index);
            },
            UpdateOp::MapClear => {
                self.clear();
            },
            _ => {
                unimplemented!();
            },
        }
        Ok(())
    }
}
