use super::*;
use std::ops::{Deref,DerefMut};
use std::io::Cursor;
use visitor::updater::CallUpdate;
use visitor::refresher::Refresher;
use node::{NodeBase, NodeContext, NewNode};

/// Manages `Connection`s and `Node`s. Does not manage sockets, 
/// this is up to the user as ggnet is net API agnostic.
pub struct Server {
    context: Arc<Mutex<NodeContext<TagServer>>>,
    clients: Vec<(Connection, Box<NodeBase<TagServer>>)>,
    next_connection_id: usize,
    next_node_id: u32,
}

/// A managed connection to a `Server`. Does not manage sockets, 
/// this is up to the user as ggnet is net API agnostic.
/// The `Client` expects that the server assigns a `Node` of type `Node<T, _>` to the connection.
/// The `Client` implements `Deref` and `DerefMut` so the user can access the wrapped `Node<T, _>`.
pub struct Client<T: Default + CallUpdate + CallRPC + Any + Reflect<Refresher>> {
    conn: Connection,
    root: Node<T, TagClient>,
    context: Arc<Mutex<NodeContext<TagClient>>>,
}

impl Server {
    /// Initializes a new server.
    pub fn new() -> Server {
        Self {
            context: Arc::new(Mutex::new(NodeContext::new())),
            clients: Vec::new(),
            next_connection_id: 1,
            next_node_id: 1,
        }
    }

    /// Manage a new connection. 
    /// The connection will be initialized using the supplied `Write` and `Read` implementations,
    ///  which correspond to the sending and receiving end of a two way socket.
    /// The connection is also expected to have it's own root `Node`,
    ///  for which the user needs to supply a suitable contained value.
    pub fn add_client<W, R, T>(&mut self, w: W, r: R, root: T)  where
        W: 'static + Write,
        R: 'static + Read + 
                     Send,
        T: 'static + CallRPC + 
                     CallUpdate + 
                     Default + 
                     Any + 
                     Reflect<Serializer<Vec<u8>>> + 
                     Reflect<Refresher>,
    {
        let conn = Connection::new(w, r, self.next_connection_id);
        let mut root = self.make_node(root);

        let mut ser = Serializer::new(Vec::new());
        root.set_root(conn.clone());
        root.reflect(&mut ser).unwrap();
        conn.send(root.id(), ser.writer.as_slice());

        self.clients.push((conn.clone(), root.as_box()));
        self.next_connection_id += 1;
    }

    /// Create a new `Node` managed by this `Server`. 
    /// The `Server` will assign an id and keep a weak reference to it for future lookup.
    pub fn make_node<T>(&mut self, content: T) -> Node<T, TagServer> where
        T: 'static + CallUpdate + CallRPC + Default + Any + Reflect<Refresher>
    {
        let node = Node::new(self.next_node_id, content, self.context.clone());

        self.context.lock().unwrap().insert(self.next_node_id, node.clone());
        self.next_node_id += 1;

        node
    }

    /// Updates the `Server`. Processes received messages from the managed connections and 
    ///  cleans up inactive connections and their root nodes after that.
    pub fn update(&mut self) {
        let clients = &mut self.clients;
        let context = &self.context;

        for &mut (ref mut conn, _) in clients.iter_mut() {
            conn.recv().map(|packet| {
                let mut node = context.lock().unwrap().get(packet.node).unwrap().as_box();
                node.recv_rpc(Deserializer::new(Cursor::new(packet.data)));
            });
        }

        clients.retain(|(ref c, _)| c.status().is_ok());
    }
}

impl<T> Client<T> where
    T: CallUpdate + 
       CallRPC + 
       Default + 
       Any + 
       Reflect<Deserializer<Cursor<Vec<u8>>>> + 
       Reflect<Refresher>
{
    /// Initialize a new connection to the `Server`. 
    /// The supplied connection should be a live connection,
    ///  but it should not have been used for any ggnet traffic yet. 
    pub fn new(conn: Connection) -> Result<Self, Error> {
        let context = Arc::new(Mutex::new(NodeContext::new()));

        let packet = conn.recv_blocking();
        let packet = packet.ok_or_else(|| conn.status().err().unwrap())?;

        let mut de = Deserializer::new(Cursor::new(packet.data));
        de.attach_context(context.clone());

        let mut root = Node::new(packet.node, T::default(), context.clone());
        root.set_root(conn.clone());
        
        context.lock().unwrap().insert(packet.node, root.clone());
        
        root.reflect(&mut de).unwrap();
        
        assert!(root.id() > 0);

        Ok(Self { conn, root, context })
    }

    /// Update the connection. Processes any messages received from the server.
    pub fn update(&mut self) {
        loop {
            let packet = self.conn.recv();
            if packet.is_none() {
                break;
            }

            let packet = packet.unwrap();

            let mut node = self.context.lock().unwrap().get(packet.node).unwrap().as_box();
            let mut de = Deserializer::with_current_node(Cursor::new(packet.data), packet.node);
            de.attach_context(self.context.clone());
            
            node.recv_update(de);
        }
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>> Deref for Client<T> {
    type Target = Node<T, TagClient>;

    /// Provides immutable access to the wrapped `Node<T, TagClient>`.
    fn deref(&self) -> &Node<T, TagClient> {
        &self.root
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>> DerefMut for Client<T> {
    /// Provides mutable access to the wrapped `Node<T, TagClient>`.
    fn deref_mut(&mut self) -> &mut Node<T, TagClient> {
        &mut self.root
    }
}
