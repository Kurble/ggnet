use super::*;
use std::ops::{Deref,DerefMut};
use std::io::Cursor;
use std::rc::Rc;
use std::cell::RefCell;
use updater::CallUpdate;

pub struct Server {
    nodes: HashMap<u32, Box<NodeBase<TagServer>>>,
    clients: Vec<ServerClient>,
    next_connection_id: usize,
    next_node_id: u32,
}

pub struct ServerClient {
    conn: Connection,
    root: Box<NodeBase<TagServer>>,
}

pub struct Client<T: Default + CallUpdate + CallRPC + Any> {
	conn: Connection,
	root: Node<T, TagClient>,
	nodes: Rc<RefCell<HashMap<u32, Box<NodeBase<TagClient>>>>>,
}

impl Server {
	pub fn new() -> Server {
		Self {
			nodes: HashMap::new(),
			clients: Vec::new(),
			next_connection_id: 1,
			next_node_id: 1,
		}
	}

	pub fn add_client<W, R, T>(&mut self, w: W, r: R, mut root: T)  where
		W: 'static + Write,
		R: 'static + Read + Send,
		T: 'static + NodeBase<TagServer> + Default + NodeServerExt,
	{
		let conn = Connection::new(w, r, self.next_connection_id);

		let client = ServerClient{
			conn: conn.clone(),
			root: root.as_box(),
		};

		root.track(conn);
		root.resync();

		self.clients.push(client);
		self.next_connection_id += 1;
	}

	pub fn make_node<T>(&mut self, content: T) -> Node<T, TagServer> where
		T: 'static + CallUpdate + CallRPC + Default + Any
	{
		let node = Node::new(self.next_node_id, content);

		self.nodes.insert(self.next_node_id, node.as_box());
		self.next_node_id += 1;

		node
	}

	pub fn update(&mut self) {
		for c in self.clients.iter_mut() {
			c.update(&mut self.nodes);
		}
	}
}

impl<T: CallUpdate + CallRPC + Default + Any> Client<T> {
	pub fn new(conn: Connection) -> Self {
		let packet = conn.recv_blocking();

		let mut nodes = Rc::new(RefCell::new(HashMap::new()));

		let mut root = Node::new(packet.node, T::default());

		root.recv_update(Deserializer {
			reader: Cursor::new(packet.data),
			context: Box::new(nodes.clone()),
		});

		root.track(conn.clone());

		nodes.insert(packet.node, root.as_box());

		Self { conn, root, nodes }
	}

	pub fn update(&mut self) {
		loop {
			let packet = self.conn.recv();
			if packet.is_none() {
				break;
			}

			let packet = packet.unwrap();

			let mut node = self.nodes.get(packet.node).unwrap().as_box();
			
			node.recv_update(Deserializer {
				reader: Cursor::new(packet.data),
				context: Box::new(self.nodes.clone()),
			});
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

impl<T: Tag> NodeContext<T> for Rc<RefCell<HashMap<u32, Box<NodeBase<T>>>>> {
	fn get(&self, id: u32) -> Option<Box<NodeBase<T>>> {
		self.borrow().get(&id).map(|node| node.as_box())
	}
	fn insert(&mut self, id: u32, node: Box<NodeBase<T>>) {
		self.borrow_mut().insert(id, node);
	}
}

impl ServerClient {
	fn update<'a>(&'a mut self, nodes: &'a mut HashMap<u32, Box<NodeBase<TagServer>>>) {
		// read and process available packets
		self.conn.recv().map(|packet| {
			let mut node = nodes.get_mut(&packet.node).unwrap().as_box();
			
			node.recv_rpc(Deserializer {
				reader: Cursor::new(packet.data),
				context: Box::new(()),
			});
		});
	}
}