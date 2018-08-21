use super::*;
use std::ops::{Deref,DerefMut};
use std::io::Cursor;
use visitor::updater::CallUpdate;
use visitor::refresher::Refresher;

pub struct Server {
    context: Arc<Mutex<NodeContext<TagServer>>>,
    clients: Vec<ServerClient>,
    next_connection_id: usize,
    next_node_id: u32,
}

pub struct ServerClient {
    context: Arc<Mutex<NodeContext<TagServer>>>,
    conn: Connection,
    root: Box<NodeBase<TagServer>>,
}

pub struct Client<T: Default + CallUpdate + CallRPC + Any + Reflect<Refresher>> {
    conn: Connection,
    root: Node<T, TagClient>,
    context: Arc<Mutex<NodeContext<TagClient>>>,
}

impl Server {
    pub fn new() -> Server {
        Self {
            context: Arc::new(Mutex::new(NodeContext::new())),
            clients: Vec::new(),
            next_connection_id: 1,
            next_node_id: 1,
        }
    }

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

        self.clients.push(ServerClient{
            context: self.context.clone(),
            conn: conn.clone(),
            root: root.as_box(),
        });
        self.next_connection_id += 1;
    }

    pub fn make_node<T>(&mut self, content: T) -> Node<T, TagServer> where
        T: 'static + CallUpdate + CallRPC + Default + Any + Reflect<Refresher>
    {
        let node = Node::new(self.next_node_id, content, self.context.clone());

        self.context.lock().unwrap().insert(self.next_node_id, node.clone());
        self.next_node_id += 1;

        node
    }

    pub fn update(&mut self) {
        self.clients.retain(|c| c.conn.is_alive());
        for c in self.clients.iter_mut() {
            c.update();
        }
    }
}

impl ServerClient {
    fn update<'a>(&'a mut self) {
        self.conn.recv().map(|packet| {
            let mut node = self.context.lock().unwrap().get(packet.node).unwrap().as_box();
            node.recv_rpc(Deserializer::new(Cursor::new(packet.data)));
        });
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
    pub fn new(conn: Connection) -> Self {
        let context = Arc::new(Mutex::new(NodeContext::new()));

        let packet = conn.recv_blocking();

        let mut root = Node::new(packet.node, T::default(), context.clone());
        root.set_root(conn.clone());
        context.lock().unwrap().insert(packet.node, root.clone());
        let mut de = Deserializer::new(Cursor::new(packet.data));
        de.attach_context(context.clone());
        root.reflect(&mut de).unwrap();
        assert!(root.id() > 0);

        Self { conn, root, context }
    }

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

    fn deref(&self) -> &Node<T, TagClient> {
        &self.root
    }
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Refresher>> DerefMut for Client<T> {
    fn deref_mut(&mut self) -> &mut Node<T, TagClient> {
        &mut self.root
    }
}
