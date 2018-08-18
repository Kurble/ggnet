use super::*;
use std::ops::{Deref,DerefMut};
use std::io::Cursor;
use visitor::updater::CallUpdate;

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

pub struct Client<T: Default + CallUpdate + CallRPC + Any> {
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

	pub fn add_client<W, R, T>(&mut self, w: W, r: R, mut root: Node<T, TagServer>)  where
		W: 'static + Write,
		R: 'static + Read + Send,
		T: 'static + CallRPC + CallUpdate + Default + Any,
		Node<T, TagServer>: NodeServerExt
	{
		let conn = Connection::new(w, r, self.next_connection_id);

		let mut ser = Serializer::new(Vec::new());
		root.reflect(&mut ser).unwrap();
		conn.send(0, ser.writer.as_slice());
		root.set_root(conn.clone());

		self.clients.push(ServerClient{
			context: self.context.clone(),
			conn: conn.clone(),
			root: root.as_box(),
		});
		self.next_connection_id += 1;
	}

	pub fn make_node<T>(&mut self, content: T) -> Node<T, TagServer> where
		T: 'static + CallUpdate + CallRPC + Default + Any
	{
		let node = Node::new(self.next_node_id, content, self.context.clone());

		self.context.lock().unwrap().insert(self.next_node_id, node.as_box());
		self.next_node_id += 1;

		node
	}

	pub fn update(&mut self) {
		self.clients.retain(|c| if c.conn.is_alive() { true } else { println!("drop connection"); false });

		for c in self.clients.iter_mut() {
			c.update();
		}
	}
}

impl<T: CallUpdate + CallRPC + Default + Any + Reflect<Deserializer<Cursor<Vec<u8>>>>> Client<T> {
	pub fn new(conn: Connection) -> Self {
		let context = Arc::new(Mutex::new(NodeContext::new()));

		let packet = conn.recv_blocking();

		let mut root = Node::new(packet.node, T::default(), context.clone());
		root.reflect(&mut Deserializer::new(Cursor::new(packet.data))).unwrap();
		root.set_root(conn.clone());
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
			
			node.recv_update(Deserializer::new(Cursor::new(packet.data)));
		}
	}
}

impl<T: CallUpdate + CallRPC + Default + Any> Deref for Client<T> {
	type Target = Node<T, TagClient>;

	fn deref(&self) -> &Node<T, TagClient> {
		&self.root
	}
}

impl<T: CallUpdate + CallRPC + Default + Any> DerefMut for Client<T> {
	fn deref_mut(&mut self) -> &mut Node<T, TagClient> {
		&mut self.root
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