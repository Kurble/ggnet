use super::*;
use std::io::Cursor;
use std::ops::{Deref,DerefMut};
use std::mem::replace;
use std::sync::MutexGuard;
use std::collections::HashSet;
use visitor::updater::{Updater, CallUpdate};
use visitor::refresher::Refresher;

pub type BufferSerializer = Serializer<Vec<u8>>;

pub type BufferDeserializer = Deserializer<Cursor<Vec<u8>>>;

pub trait Tag: 'static + Default { }

#[derive(Default)]
pub struct TagServer;

#[derive(Default)]
pub struct TagClient;

#[derive(Default)]
pub struct TagAgnostic;

impl Tag for TagServer { }

impl Tag for TagClient { }

impl Tag for TagAgnostic { }

pub trait NodeBase<T: Tag>: Any {
    fn as_box(&self) -> Box<NodeBase<T>>;
    fn as_any(&self) -> &Any;
    fn id(&self) -> u32;
    fn send(&self, BufferSerializer);
    fn recv_rpc<'a>(&mut self, BufferDeserializer);
    fn recv_update<'a>(&mut self, BufferDeserializer);
    fn add_ref(&mut self, parent: u32);
    fn remove_ref(&mut self, parent: u32);
    fn add_connections(&self, target: &mut HashSet<Connection>); 
}

pub struct NodeContext<T: Tag> {
    nodes: HashMap<u32, Box<NodeBase<T>>>,
}

struct NodeInner {
    refs: HashSet<u32>,
	conns: HashSet<Connection>,
}

pub struct Node<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: 'static + Tag> {
    owner: Option<u32>,
    id: u32,
    context: Option<Arc<Mutex<NodeContext<G>>>>,
    val: Arc<Mutex<T>>,
    inner: Arc<Mutex<NodeInner>>,
    root: Option<Connection>,
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: 'static + Tag> Drop for Node<T, G> {
    fn drop(&mut self) {
        if self.owner.is_some() {
            let owner = self.owner.unwrap();
            self.remove_ref(owner);
        }
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> Clone for Node<T, G> {
    fn clone(&self) -> Self {
        Self {
            owner: None,
            id: self.id,
            context: self.context.clone(),
            val: self.val.clone(),
            inner: self.inner.clone(),
            root: self.root.clone(),
        }
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> Default for Node<T, G> {
    fn default() -> Self {
        Self {
        	owner: None,
            id: 0,
            context: None,
            val: Arc::new(Mutex::new(Default::default())),
        	inner: Arc::new(Mutex::new(NodeInner {
        		refs: HashSet::new(),
        		conns: HashSet::new(),
        	})),
            root: None,
        }
    }
}

/*

big hashmap with all cumulative connections


HashMap<NodeID, (Set<Conn> /* cumulative */, Vec<NodeID> /*ref-by*/)>
0 -> [conn 0], []
1 -> [conn 1], []
3 -> [conn 2], []

2 -> [conn 0, conn 1], [0, 1]

remove 2 from 1: 
-> remove all 2's connections
-> remove 1 from 2's ref-by list
-> gather all connections from ref-by list
-> recurse to 2's children:
    -> regather connections

add 2 to 3
-> add 3 to 2's ref-by list
-> add new connections from 3
-> recurse to 2's children:
    -> regather connections


every node is referenced by an Arc<Inner>...
should Inner contain a list of parent id's?

Node.drop could then deregister itself as a parent



*/





impl<W, T, G> Reflect<Serializer<W>> for Node<T,G> where
    W: Write,
    T: 'static + CallUpdate + CallRPC + Reflect<Serializer<W>> + Reflect<Refresher>,
    G: 'static + Tag
{
    fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), SerializeError> {
        self.owner = visit.current_node.clone();
        visit.current_node.as_ref().map(|id| self.add_ref(*id));

        // push a new parent id on the stack
        let parent = replace(&mut visit.current_node, Some(self.id));
        // reflect using the new parent
        self.id.reflect(visit)?;
        self.val.lock().unwrap().reflect(visit)?;
        // return to the old parent
        visit.current_node = parent;

        Ok(())
    }
}

impl<R, T, G> Reflect<Deserializer<R>> for Node<T,G> where
    R: Read,
    T: 'static + CallUpdate + CallRPC + Reflect<Deserializer<R>> + Reflect<Refresher>,
    G: 'static + Tag,
{
    fn reflect(&mut self, visit: &mut Deserializer<R>) -> Result<(), SerializeError> {
        self.id.reflect(visit)?;

        let node = self.context.as_ref().unwrap().lock().unwrap().get(self.id);

        let shared_inner: Box<NodeBase<G>> = node.unwrap_or_else(|| {
            let new_node = Node::<T, G>::new(0, Default::default(), self.context.as_ref().unwrap().clone());

            self.context.as_ref().unwrap().lock().unwrap().insert(self.id, new_node.as_box());
            new_node.as_box()
        });
        let shared_inner = shared_inner.as_any().downcast_ref::<Self>().unwrap();
        self.inner = shared_inner.inner.clone();
        self.val = shared_inner.val.clone();

        self.val.lock().unwrap().reflect(visit)?;

        Ok(())
    }
}

impl<V, T, G> Reflect<Updater<V>> for Node<T, G> where
    V: Visitor,
    T: 'static + CallUpdate + CallRPC + Reflect<Updater<V>> + Reflect<Refresher>,
    G: Tag,
{
    fn reflect(&mut self, visit: &mut Updater<V>) -> Result<(), SerializeError> {
        self.val.lock().unwrap().reflect(visit)?;
        Ok(())
    }
}

impl<T, G> Reflect<Refresher> for Node<T,G> where
    T: 'static + CallUpdate + CallRPC + Reflect<Refresher>,
    G: 'static + Tag
{
    fn reflect(&mut self, visit: &mut Refresher) -> Result<(), SerializeError> {
        {
            self.context = Some(visit.context());

            let mut inner = self.inner.lock().unwrap();
            let inner: &mut NodeInner = &mut inner;
            let refs = &mut inner.refs;
            let conns = &mut inner.conns;
            let context = self.context.as_ref().unwrap().lock().unwrap();

            conns.clear();
            
            for c in self.root.iter() {
                conns.insert(c.clone());
            }

            for r in refs.iter() {
                context.get(*r).unwrap().add_connections(conns);
            }
        }

        Ok(self.val.lock().unwrap().reflect(visit)?)
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> Node<T, G> {
    pub fn new(id: u32, val: T, context: Arc<Mutex<NodeContext<G>>>) -> Self {
        Self {
            owner: None,
            id,
            context: Some(context),
            val: Arc::new(Mutex::new(val)),
            inner: Arc::new(Mutex::new(NodeInner{
                refs: HashSet::new(),
                conns: HashSet::new()
            })),
            root: None,
        }
    }

    pub fn set_root(&mut self, conn: Connection) {
        self.root = Some(conn.clone());
        self.inner.lock().unwrap().conns.insert(conn.clone());
    }

    fn inner_clone(&self) -> Box<Node<T, G>> {
        Box::new(Node {
            owner: None,
            id: self.id,
            context: self.context.clone(),
            val: self.val.clone(),
            inner: self.inner.clone(),
            root: self.root.clone(),
        })
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: 'static + Tag> NodeBase<G> for Node<T, G> {
    fn as_box(&self) -> Box<NodeBase<G>> { self.inner_clone() }

    fn as_any(&self) -> &Any {
        self
    }

    fn id(&self) -> u32 { self.id }

    fn send(&self, msg: BufferSerializer) {
        for conn in self.inner.lock().unwrap().conns.iter() {
            conn.send(self.id, msg.writer.as_slice());
        }
    }

    fn recv_rpc<'a>(&mut self, msg: BufferDeserializer) {
        T::call_rpc(self, msg);
    }

    fn recv_update<'a>(&mut self, msg: BufferDeserializer) {
        self.val.lock().unwrap().call_upd(msg);
    }

    fn add_ref(&mut self, parent: u32) {
        {
            let mut inner = self.inner.lock().unwrap();
            let context = self.context.as_ref().unwrap().lock().unwrap();

            inner.refs.insert(parent);
            context.get(parent).unwrap().add_connections(&mut inner.conns);
        }

        self.val.lock().unwrap().reflect(&mut Refresher::new(self.context.clone().unwrap())).unwrap();
    }

    fn remove_ref(&mut self, parent: u32) {
        {
            let mut inner = self.inner.lock().unwrap();
            let inner: &mut NodeInner = &mut inner;
            let conns = &mut inner.conns;
            let refs = &mut inner.refs;
            let context = self.context.as_ref().unwrap().lock().unwrap();

            conns.clear();
            refs.retain(|x| x != &parent);
            for r in refs.iter() {
                context.get(*r).unwrap().add_connections(conns);
            }
        }

        self.val.lock().unwrap().reflect(&mut Refresher::new(self.context.clone().unwrap())).unwrap();
    }

    fn add_connections(&self, target: &mut HashSet<Connection>) {
        self.root.as_ref().map(|root| target.insert(root.clone()));
        for c in self.inner.lock().unwrap().conns.iter() {
            target.insert(c.clone());
        }
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> Node<T, G> {
	pub fn as_ref<'a>(&'a self) -> Borrow<'a, T> {
		Borrow { x: self.val.lock().unwrap() }
	}

	pub fn as_mut<'a>(&'a mut self) -> BorrowMut<'a, T> {
		BorrowMut { x: self.val.lock().unwrap() }
	}
}

pub struct Borrow<'a, T: 'a> {
	x: MutexGuard<'a, T>,
}

pub struct BorrowMut<'a, T: 'a> {
	x: MutexGuard<'a, T>,
}

impl<'a, T: Default> Deref for Borrow<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.x
	}
}

impl<'a, T: Default> Deref for BorrowMut<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.x
	}
}

impl<'a, T: Default> DerefMut for BorrowMut<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut self.x
	}
}

impl<T: Tag> NodeContext<T> {
    pub fn new() -> NodeContext<T> {
        Self {
            nodes: HashMap::new()
        }
    }

    pub fn get(&self, id: u32) -> Option<Box<NodeBase<T>>> {
        self.nodes.get(&id).map(|node| node.as_box())
    }

    pub fn insert(&mut self, id: u32, node: Box<NodeBase<T>>) {
        self.nodes.insert(id, node);
    }
}