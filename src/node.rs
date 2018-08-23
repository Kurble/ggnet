use super::*;
use std::io::Cursor;
use std::ops::{Deref,DerefMut};
use std::mem::replace;
use std::sync::{Weak, MutexGuard};
use std::collections::HashSet;
use visitor::updater::{Updater, CallUpdate};
use visitor::refresher::Refresher;

pub type BufferSerializer = Serializer<Vec<u8>>;

pub type BufferDeserializer = Deserializer<Cursor<Vec<u8>>>;

/// Tags used to specialize `Node` implementations between server sided and client sided.
/// See `TagServer` and `TagClient`.
pub trait Tag: 'static + Default { }

/// Tag used for `Node`s on the `Server`.
#[derive(Default)]
pub struct TagServer;

/// Tag used for `Node`s on the `Client`.
#[derive(Default)]
pub struct TagClient;

#[derive(Default)]
pub struct TagAgnostic;

impl Tag for TagServer { }

impl Tag for TagClient { }

impl Tag for TagAgnostic { }

/// Trait for dynamic dispatch to `Node`s.
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
    root: Option<Connection>,
}

/// A node. Entry point for server <--> client communication. 
pub struct Node<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: 'static + Tag> {
    owner: Option<u32>,
    id: u32,
    context: Option<Arc<Mutex<NodeContext<G>>>>,
    val: Arc<Mutex<T>>,
    inner: Arc<Mutex<NodeInner>>,
}

struct WeakNode<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> {
    id: u32,
    context: Option<Arc<Mutex<NodeContext<G>>>>,
    val: Weak<Mutex<T>>,
    inner: Weak<Mutex<NodeInner>>,
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: 'static + Tag> Drop for Node<T, G> {
    fn drop(&mut self) {
        if self.owner.is_some() {
            let owner = self.owner.unwrap();
            self.remove_ref(owner);
        }

        // destroy the node if this is the last strong reference
        if Arc::strong_count(&self.inner) == 1 {
            println!("gc node {}", self.id);
            self.context.as_ref().unwrap().lock().unwrap().gc(self.id);
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
                root: None,
            })),
        }
    }
}

impl<W, T, G> Reflect<Serializer<W>> for Node<T,G> where
    W: Write,
    T: 'static + CallUpdate + CallRPC + Reflect<Serializer<W>> + Reflect<Refresher>,
    G: 'static + Tag
{
    fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), Error> {
        self.id.reflect(visit)?;

        self.owner = visit.current_node.clone();
        visit.current_node.as_ref().map(|id| self.add_ref(*id));

        // push a new parent id on the stack
        let parent = replace(&mut visit.current_node, Some(self.id));
        // reflect using the new parent
        self.val.lock().unwrap().reflect(visit)?;
        // return to the old parent
        visit.current_node = parent;

        Ok(())
    }
}

impl<R, T, G> Reflect<Deserializer<R>> for Node<T,G> where
    R: Read,
    T: 'static + CallUpdate + CallRPC + Reflect<Deserializer<R>> + Reflect<Refresher>,
    G: Tag,
{
    fn reflect(&mut self, visit: &mut Deserializer<R>) -> Result<(), Error> {
        self.id.reflect(visit)?;

        if self.context.is_none() {
            self.context = Some(visit.context());
        }

        // resolve node content
        let node = self.context.as_ref().unwrap().lock().unwrap().get(self.id);
        let shared_inner: Box<NodeBase<G>> = node.unwrap_or_else(|| {
            let new_node = Node::<T, G>::new(0, T::default(), self.context.clone().unwrap());
            let result = new_node.as_box();
            self.context.as_ref().unwrap().lock().unwrap().insert(self.id, new_node);
            result            
        });
        let shared_inner = shared_inner.as_any().downcast_ref::<Self>().unwrap();
        self.inner = shared_inner.inner.clone();
        self.val = shared_inner.val.clone();
        // set node owner
        self.owner = visit.current_node.clone();
        visit.current_node.as_ref().map(|id| self.add_ref(*id));

        // push a new parent id on the stack
        let parent = replace(&mut visit.current_node, Some(self.id));
        // reflect using the new parent
        self.val.lock().unwrap().reflect(visit)?;
        // return to the old parent
        visit.current_node = parent;

        Ok(())
    }
}

impl<V, T, G> Reflect<Updater<V>> for Node<T, G> where
    V: Visitor,
    T: 'static + CallUpdate + CallRPC + Reflect<Updater<V>> + Reflect<Refresher>,
    G: Tag,
{
    fn reflect(&mut self, visit: &mut Updater<V>) -> Result<(), Error> {
        self.val.lock().unwrap().reflect(visit)?;
        Ok(())
    }
}

impl<T, G> Reflect<Refresher> for Node<T,G> where
    T: 'static + CallUpdate + CallRPC + Reflect<Refresher>,
    G: 'static + Tag
{
    fn reflect(&mut self, visit: &mut Refresher) -> Result<(), Error> {
        {
            let mut inner = self.inner.lock().unwrap();
            let inner: &mut NodeInner = &mut inner;
            let refs = &mut inner.refs;
            let conns = &mut inner.conns;
            let root = &inner.root;
            let context = self.context.as_ref().unwrap().lock().unwrap();

            conns.clear();
            
            for c in root.iter() {
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
                conns: HashSet::new(),
                root: None,
            })),
        }
    }

    pub fn set_root(&mut self, conn: Connection) {
        let mut inner = self.inner.lock().unwrap();
        inner.root = Some(conn.clone());
        inner.conns.insert(conn.clone());
    }

    fn inner_clone(&self) -> Box<Node<T, G>> {
        Box::new(Node {
            owner: None,
            id: self.id,
            context: self.context.clone(),
            val: self.val.clone(),
            inner: self.inner.clone(),
        })
    }
}

impl<T, G> NodeBase<G> for WeakNode<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>,
    G: Tag
{
    fn as_box(&self) -> Box<NodeBase<G>> { 
        Box::new(Node {
            owner: None,
            id: self.id,
            context: self.context.clone(),
            val: Weak::upgrade(&self.val).unwrap(),
            inner: Weak::upgrade(&self.inner).unwrap(),
        })
    }

    fn as_any(&self) -> &Any { self }

    fn id(&self) -> u32 { self.id }

    fn send(&self, _: BufferSerializer) { unimplemented!(); }

    fn recv_rpc<'a>(&mut self, msg: BufferDeserializer) { self.as_box().recv_rpc(msg); }

    fn recv_update<'a>(&mut self, msg: BufferDeserializer) { self.as_box().recv_update(msg); }

    fn add_ref(&mut self, _: u32) { unimplemented!(); }

    fn remove_ref(&mut self, _: u32) { unimplemented!(); }

    fn add_connections(&self, target: &mut HashSet<Connection>) { self.as_box().add_connections(target); }
}

impl<T, G> NodeBase<G> for Node<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, 
    G: Tag
{
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
        assert!(parent != self.id);

        {
            let mut inner = self.inner.lock().unwrap();
            let context = self.context.as_ref().unwrap().lock().unwrap();
            inner.refs.insert(parent);
            context.get(parent).unwrap().add_connections(&mut inner.conns);
        }

        self.val.lock().unwrap().reflect(&mut Refresher).unwrap();
    }

    fn remove_ref(&mut self, parent: u32) {
        assert!(parent != self.id);

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

        self.val.lock().unwrap().reflect(&mut Refresher).unwrap();
    }

    fn add_connections(&self, target: &mut HashSet<Connection>) {
        let inner = self.inner.lock().unwrap();
        inner.root.as_ref().map(|root| {
            target.insert(root.clone())
        });
        for c in inner.conns.iter() {
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

impl<G: Tag> NodeContext<G> {
    pub fn new() -> NodeContext<G> {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn get(&self, id: u32) -> Option<Box<NodeBase<G>>> {
        self.nodes.get(&id).map(|node| node.as_box())
    }

    pub fn insert<T>(&mut self, id: u32, node: Node<T,G>) where
        T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>
    {
        // only keep weak nodes in the node context so we can remove them
        //  when they're not needed anymore
        self.nodes.insert(id, Box::new(WeakNode {
            id: node.id.clone(),
            context: node.context.clone(),
            inner: Arc::downgrade(&node.inner),
            val: Arc::downgrade(&node.val),
        }));
    }

    pub fn gc(&mut self, id: u32) {
        self.nodes.remove(&id);
    }
}