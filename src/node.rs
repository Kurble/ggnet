
use super::*;
use std::io::Cursor;
use std::ops::{Deref,DerefMut};
use std::mem::replace;
use std::sync::{Weak, MutexGuard};
use std::collections::HashSet;
use visitor::updater::{Updater, UpdateOp, CallUpdate};
use visitor::refresher::Refresher;
use visitor::printer::Printer;

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
    fn context(&self) -> Arc<Mutex<NodeContext<T>>>;
    fn send(&self, BufferSerializer);
    fn recv_rpc<'a>(&mut self, BufferDeserializer);
    fn recv_update<'a>(&mut self, BufferDeserializer);
    fn add_ref(&mut self, parent: u32);
    fn remove_ref(&mut self, parent: u32);
    fn add_connections(&self, target: &mut HashSet<Connection>); 
}

/// Private functions for `Node<T,G>`
pub trait NewNode<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, G: Tag> {
    fn new(id: u32, val: T, context: Arc<Mutex<NodeContext<G>>>) -> Self;
    fn set_root(&mut self, conn: Connection);
}

pub struct NodeContext<T: Tag> {
    nodes: HashMap<u32, Box<NodeBase<T>>>,
    next: u32,
}

struct NodeInner {
    refs: HashSet<u32>,
    conns: HashSet<Connection>,
    root: Option<Connection>,
    changed: bool,
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

impl<T, G> Drop for Node<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, 
    G: Tag 
{
    fn drop(&mut self) {
        if self.id > 0 {
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
}

impl<T, G> Clone for Node<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, 
    G: Tag 
{
    fn clone(&self) -> Self {
        assert!(self.id > 0);
        Self {
            owner: None,
            id: self.id,
            context: self.context.clone(),
            val: self.val.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<T, G> Default for Node<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, 
    G: Tag 
{
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
                changed: false,
            })),
        }
    }
}

impl<T, G> NewNode<T,G> for Node<T,G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>,
    G: Tag
{
    fn new(id: u32, val: T, context: Arc<Mutex<NodeContext<G>>>) -> Self {
        assert!(id > 0);

        Self {
            owner: None,
            id,
            context: Some(context),
            val: Arc::new(Mutex::new(val)),
            inner: Arc::new(Mutex::new(NodeInner{
                refs: HashSet::new(),
                conns: HashSet::new(),
                root: None,
                changed: false,
            })),
        }
    }

    fn set_root(&mut self, conn: Connection) {
        let mut inner = self.inner.lock().unwrap();
        inner.root = Some(conn.clone());
        inner.conns.insert(conn.clone());
    }
}

impl<W, T, G> Reflect<Serializer<W>> for Node<T,G> where
    W: Write,
    T: 'static + CallUpdate + CallRPC + Reflect<Serializer<W>> + Reflect<Refresher>,
    G: 'static + Tag
{
    fn reflect(&mut self, visit: &mut Serializer<W>) -> Result<(), Error> {
        self.id.reflect(visit)?;

        assert!(self.id > 0);

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

        assert!(self.id > 0);

        if self.context.is_none() {
            self.context = Some(visit.context());
        }

        // resolve node content
        let node = self.context.as_ref().unwrap().lock().unwrap().get(self.id);
        if node.is_some() {
            println!("Found old node with id{}", self.id);
        }

        let shared_inner: Box<NodeBase<G>> = node.unwrap_or_else(|| {
            println!("Creating new node with id {}", self.id);
            let new_node = Node::<T, G>::new(self.id, T::default(), self.context.clone().unwrap());
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
    T: 'static + CallUpdate + CallRPC + Reflect<Updater<V>> + Reflect<Refresher> + Reflect<V>,
    G: Tag,
    u32: Reflect<Updater<V>>,
{
    fn reflect(&mut self, visit: &mut Updater<V>) -> Result<(), Error> {
        let old_id = self.id;
        self.id.reflect(visit)?;

        println!("reflect node {}", self.id);

        assert!(self.id > 0);

        if self.context.is_none() {
            self.context = Some(visit.context());
        }

        if old_id != self.id {
            println!("resolve inner data ({} -> {})", old_id, self.id);

            // resolve inner data
            let node = self.context.as_ref().unwrap().lock().unwrap().get(self.id);
            let shared_inner: Box<NodeBase<G>> = node.unwrap_or_else(|| {
                println!("Creating new node with id {}", self.id);
                let new_node = Node::<T, G>::new(self.id, T::default(), self.context.clone().unwrap());
                let result = new_node.as_box();
                self.context.as_ref().unwrap().lock().unwrap().insert(self.id, new_node);
                result            
            });
            let shared_inner = shared_inner.as_any().downcast_ref::<Self>().unwrap();
            self.inner = shared_inner.inner.clone();
            self.val = shared_inner.val.clone();
            // set node owner
            self.owner = Some(visit.current_node.clone());
        }
        
        // push a new parent id on the stack
        let parent = replace(&mut visit.current_node, self.id);
        // reflect using the new parent
        self.val.lock().unwrap().reflect(visit)?;
        // return to the old parent
        visit.current_node = parent;

        if old_id != self.id {
            // add references AFTER reflecting, so that child nodes can be initialized.
            self.add_ref(visit.current_node);
        }

        Ok(())
    }
}

impl<T, G> Reflect<Printer> for Node<T, G> where
    T: 'static + CallUpdate + CallRPC + Reflect<Printer> + Reflect<Refresher>,
    G: Tag,
{
    fn reflect(&mut self, visit: &mut Printer) -> Result<(), Error> {
        visit.indent.push_str("  ");
        visit.result.push_str(&format!("Node<{}> {{\n{}", self.id, visit.indent));
        self.val.lock().unwrap().reflect(visit)?;
        visit.indent.pop();
        visit.indent.pop();
        visit.result.push_str(&format!("\n{}}}\n{}", visit.indent, visit.indent));
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

impl<T, G> Node<T, G> where
    T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>, 
    G: Tag 
{
    /// Returns `true` if the contained value of this node has had any updates since the last call
    ///  of this function. Returns `false` otherwise.
    pub fn changed(&self) -> bool {
        replace(&mut self.inner.lock().unwrap().changed, false)
    }

    /// Borrow the inner value by reference.
    pub fn as_ref<'a>(&'a self) -> Borrow<'a, T> {
        Borrow { x: self.val.lock().unwrap() }
    }

    /// Borrow the inner value by mutable reference.
    pub fn as_mut<'a>(&'a mut self) -> BorrowMut<'a, T> {
        BorrowMut { x: self.val.lock().unwrap() }
    }

    /// Convert node to a different tag. This must be explicit so there is no `Into` implementation.
    /// Will panic if the tags do not actually match.
    pub fn convert<X: Tag>(self) -> Node<T, X> {
        let result: Box<Node<T, X>> = (Box::new(self) as Box<Any>)
            .downcast()
            .expect("called convert on Node<T,G> with wrong tag");
        *result
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

impl<X> Node<X, TagServer> where
    X: CallUpdate + 
       CallRPC + 
       Default + 
       Any + 
       Reflect<Refresher> + 
       Reflect<Updater<Serializer<Vec<u8>>>> +
       Reflect<Serializer<Vec<u8>>>,
{
    /// Create a new `Node` managed by this node's `Server`. 
    /// The `Server` will assign an id and keep a weak reference to it for future lookup.
    pub fn make_node<T, G>(&mut self, content: T) -> Node<T, G> where
        T: 'static + CallUpdate + CallRPC + Default + Any + Reflect<Refresher>,
        G: Tag
    {
        NodeContext::<TagServer>::create(&self.context(), content).convert()
    }

    /// Update all members.
    pub fn resync(&mut self) {
        let mut op = UpdateOp::Replace;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        String::from("root").reflect(&mut ser).unwrap();
        let mut updater = Updater::new_replace(self.id(), ser);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Update a single member with name `tag`.
    pub fn member_modified(&mut self, mut tag: String) {
        let mut op = UpdateOp::Update;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_update(self.id(), ser, tag);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Push a new element to the `Vec<T>` member with name `tag`.
    pub fn member_vec_push<T>(&mut self, mut tag: String, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecPush;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_push(self.id(), ser, tag, val);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Insert a new element to the `Vec<T>` member with name `tag` at position `index`.
    pub fn member_vec_insert<T>(&mut self, mut tag: String, index: usize, val: T) where
        T: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::VecInsert;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_insert(self.id(), ser, tag, index as u32, val);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Remove an element from the `Vec<T>` member with name `tag` at position `index`.
    pub fn member_vec_remove(&mut self, mut tag: String, index: usize) {
        let mut op = UpdateOp::VecRemove;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());
    
        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_remove(self.id(), ser, tag, index as u32);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Clear the `Vec<T>` member with name `tag`.
    pub fn member_vec_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::VecClear;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_vec_clear(self.id(), ser, tag);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Insert a new element to the `HashMap<K,V>` member with name `tag`.
    pub fn member_map_insert<K, V>(&mut self, mut tag: String, key: K, val: V) where 
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any,
        V: Reflect<Serializer<Vec<u8>>> + Any
    {
        let mut op = UpdateOp::MapInsert;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_insert(self.id(), ser, tag, key, val);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Remove an element from the `Vec<T>` member with name `tag` with key `key`.
    pub fn member_map_remove<K>(&mut self, mut tag: String, key: K) where
        K: Reflect<Serializer<Vec<u8>>> + Eq + Hash + Clone + Any
    {
        let mut op = UpdateOp::MapRemove;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());
        
        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_remove(self.id(), ser, tag, key);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
    }

    /// Clear the `HashMap<K,V>` member with name `tag`.
    pub fn member_map_clear(&mut self, mut tag: String) {
        let mut op = UpdateOp::MapClear;
        let mut ser = BufferSerializer::with_current_node(vec![], self.id());

        op.reflect(&mut ser).unwrap();
        tag.reflect(&mut ser).unwrap();
        let mut updater = Updater::new_map_clear(self.id(), ser, tag);
        self.val.lock().unwrap().reflect(&mut updater).unwrap();
        self.send(updater.unwrap());
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

    fn context(&self) -> Arc<Mutex<NodeContext<G>>> { self.context.clone().unwrap() }

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

    fn context(&self) -> Arc<Mutex<NodeContext<G>>> { self.context.clone().unwrap() }

    fn send(&self, msg: BufferSerializer) {
        let inner = self.inner.lock().unwrap();
        for conn in inner.conns.iter() {
            conn.send(self.id, msg.writer.as_slice());
        }
    }

    fn recv_rpc<'a>(&mut self, msg: BufferDeserializer) {
        T::call_rpc(self, msg);
    }

    fn recv_update<'a>(&mut self, msg: BufferDeserializer) {
        self.val.lock().unwrap().call_upd(self.id, msg);
        self.inner.lock().unwrap().changed = true;
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
            next: 1,
        }
    } 

    pub fn create<T>(context: &Arc<Mutex<Self>>, val: T) -> Node<T, G> where
        T: 'static + CallUpdate + CallRPC + Default + Any + Reflect<Refresher>
    {
        let cloned = context.clone();
        let mut context = context.lock().unwrap();
        let id = context.next;
        let node = Node::new(id, val, cloned);

        context.nodes.insert(id, Box::new(WeakNode {
            id: node.id.clone(),
            context: node.context.clone(),
            inner: Arc::downgrade(&node.inner),
            val: Arc::downgrade(&node.val),
        }));
        context.next += 1;

        node
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