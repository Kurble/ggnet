#[macro_use] extern crate ggnet_derive;
#[macro_use] extern crate ggnet;

use std::env;
use std::io::Cursor;
use ggnet::*;

use std::net::{TcpListener,TcpStream};

#[derive(Reflect, Default)]
pub struct ExampleNode {
	pub first: String,
	pub second: u32,
	pub third: String,
}

rpc! {
	ExampleNode | ExampleRPC {
		rpc test_rpc(&mut self, message: String) {
			println!("hello from a client: {}", message);
		}
	}
}

const ADDR: &str = "127.0.0.1:1337";

pub fn main() {
	let args: Vec<String> = env::args().collect();
	if args[1] == "server" {
		server_main();
	} else {
		client_main();
	}
}

pub fn client_main() {
	let stream = TcpStream::connect(ADDR).unwrap();

	println!("connected to server");

	let mut client = Client::<ExampleNode>::new(
		Connection::new(stream.try_clone().unwrap(), stream.try_clone().unwrap(),0)
	);

	println!("root received from server");

	println!("root.first = {}", client.as_ref().first);

	client.test_rpc("some long message sent from the client to the server".into());
}

pub fn server_main() {
	let mut server = Server::new();

	let listener = TcpListener::bind(ADDR).unwrap();

	listener.set_nonblocking(true).unwrap();

	println!("now listening on {}", ADDR);

	loop {
		listener.accept().map(|(stream, _)| {
			let node = server.make_node(ExampleNode {
				first: String::from("hoi"),
				second: 12,
				third: String::from("doei"),
			});

			server.add_client(
				stream.try_clone().unwrap(), 
				stream.try_clone().unwrap(),
				node);
		}).ok();

		server.update();		
	}
}