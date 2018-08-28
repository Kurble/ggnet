use super::*;
use std::io::Cursor;
use std::collections::HashMap;
use std::any::Any;
use node::{BufferDeserializer, NodeContext};

#[derive(Clone, Copy, PartialEq, Eq, Reflect, Debug)]
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
    pub ser: V,
    pub tag: String,
    pub op: UpdateOp,
    pub found: u32,
    pub nest: u32,
    pub key: Option<Box<Any>>,
    pub val: Option<Box<Any>>,
    pub current_node: u32,
    pub context: Option<Box<Any>>,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
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
            context: None,
        }
    }
    pub fn unwrap(self) -> U {
        self.ser
    }

    pub fn attach_context<G: Tag>(&mut self, context: Arc<Mutex<NodeContext<G>>>) {
        self.context = Some(Box::new(context));
    }

    pub fn context<G: Tag>(&self) -> Arc<Mutex<NodeContext<G>>> {
        self.context
            .as_ref().unwrap()
            .downcast_ref::<Arc<Mutex<NodeContext<G>>>>().unwrap()
            .clone()
    }
}

pub trait CallUpdate {
    fn call_upd(&mut self, parent: u32, msg: BufferDeserializer);
}

impl<T> CallUpdate for T where
    T: Reflect<Deserializer<Cursor<Vec<u8>>>> + 
       Reflect<Updater<Deserializer<Cursor<Vec<u8>>>>>
{
    fn call_upd(&mut self, parent: u32, mut msg: BufferDeserializer) {
        let mut op = UpdateOp::default();
        let mut tag = String::default();

        op.reflect(&mut msg).unwrap();
        tag.reflect(&mut msg).unwrap();

        println!("update op {:?} on tag {}", op, tag);

        let ctx = msg.context();

        let mut upd = Updater {
            ser: msg,
            tag,
            op,
            found: 0,
            nest: 0,
            key: None,
            val: None,
            current_node: parent,
            context: None,
        };

        upd.attach_context::<TagClient>(ctx);

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
