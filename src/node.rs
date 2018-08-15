use super::*;
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::{Deref,DerefMut};
use std::sync::MutexGuard;
use updater::CallUpdate;

pub type BufferSerializer = Serializer<Vec<u8>>;

pub type BufferDeserializer<'a, Tag> = Deserializer<&'a[u8], Tag>;

pub trait Tag { }

pub struct TagServer;

pub struct TagClient;

pub struct TagAgnostic;

impl Tag for TagServer { }

impl Tag for TagClient { }

impl Tag for TagAgnostic { }

pub trait NodeBase<T: Tag>: Any {
    fn as_box(&self) -> Box<NodeBase<T>>;
    fn id(&self) -> u32;
    fn send(&self, BufferSerializer);
    fn recv_rpc<'a>(&mut self, Deserializer<Cursor<Vec<u8>>, TagServer>);
    fn recv_update<'a>(&mut self, Deserializer<Cursor<Vec<u8>>, TagClient>);
    fn track(&mut self, conn: Connection);
    fn untrack(&mut self, conn: Connection);
}

pub trait NodeContext<T: Tag> {
	fn get(&self, id: u32) -> Option<Box<NodeBase<T>>>;
	fn insert(&mut self, id: u32, node: Box<NodeBase<T>>);
}

struct NodeInner<T> {
	val: T,
	connections: Vec<Connection>,
}

#[derive(Clone)]
pub struct Node<T: CallUpdate + CallRPC + Default + Any, G: Tag> {
    id: u32,
    inner: Arc<Mutex<NodeInner<T>>>,
    tag: PhantomData<G>,
}

impl<T: CallUpdate + CallRPC + Default + Any, G: Tag> Default for Node<T, G> {
    fn default() -> Self {
        Self {
        	id: 0,
            tag: PhantomData,
        	inner: Arc::new(Mutex::new(NodeInner {
        		val: Default::default(),
        		connections: Vec::new(),
        	})),
        }
    }
}

impl<V: Visitor, T: CallUpdate + CallRPC + Reflect<V> + Any, G: Tag> Reflect<V> for Node<T, G> {
    fn reflect(&mut self, visitor: &mut V) -> Result<(), SerializeError> {
        let mut lk = self.inner.lock().unwrap();
        Ok(lk.val.reflect(visitor)?)
    }
}

impl<T: CallUpdate + CallRPC + Default + Any, G: Tag> Node<T, G> {
    pub fn new(id: u32, val: T) -> Self {
        Self {
            id,
            tag: PhantomData,
            inner: Arc::new(Mutex::new(NodeInner{
                val,
                connections: Vec::new()
            })),
        }
    }

    fn inner_clone(&self) -> Box<Node<T, G>> {
        Box::new(Node {
            id: self.id,
            inner: self.inner.clone(),
            tag: PhantomData,
        })
    }
}



impl<T: CallUpdate + CallRPC + Default + Any, G: 'static + Tag> NodeBase<G> for Node<T, G> {
    fn as_box(&self) -> Box<NodeBase<G>> { self.inner_clone() }

    fn id(&self) -> u32 { self.id }

    fn send(&self, msg: BufferSerializer) {
        let lk = self.inner.lock().unwrap();

        // send message to recipients
        for conn in lk.connections.iter() {
            conn.send(self.id, msg.writer.as_slice());
        }
    }

    fn recv_rpc<'a>(&mut self, msg: Deserializer<Cursor<Vec<u8>>, TagServer>) {
        self.inner.lock().unwrap().val.call_rpc(msg);
    }

    fn recv_update<'a>(&mut self, msg: Deserializer<Cursor<Vec<u8>>, TagClient>) {
        self.inner.lock().unwrap().val.call_upd(msg);
    }

    fn track(&mut self, conn: Connection) {
        let mut lk = self.inner.lock().unwrap();
        lk.connections.push(conn);
    }

    fn untrack(&mut self, conn: Connection) {
        let mut lk = self.inner.lock().unwrap();
        lk.connections.retain(|x| x != &conn);
    }
}

impl<T: CallUpdate + CallRPC + Default + Any, G: Tag> Node<T, G> {
	pub fn as_ref<'a>(&'a self) -> Borrow<'a, T> {
		Borrow { x: self.inner.lock().unwrap() }
	}

	pub fn as_mut<'a>(&'a mut self) -> BorrowMut<'a, T> {
		BorrowMut { x: self.inner.lock().unwrap() }
	}
}

pub struct Borrow<'a, T: 'a> {
	x: MutexGuard<'a, NodeInner<T>>,
}

pub struct BorrowMut<'a, T: 'a> {
	x: MutexGuard<'a, NodeInner<T>>,
}

impl<'a, T: Default> Deref for Borrow<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.x.val
	}
}

impl<'a, T: Default> Deref for BorrowMut<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.x.val
	}
}

impl<'a, T: Default> DerefMut for BorrowMut<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut self.x.val
	}
}