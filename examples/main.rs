#[macro_use] extern crate ggnet_derive;
#[macro_use] extern crate ggnet;

use std::any::Any;
use std::env;
use std::io::Cursor;
use ggnet::*;
use std::time;
use std::thread;
use std::io::stdin;

use std::net::{TcpListener,TcpStream};

#[derive(Reflect, Default)]
pub struct ExampleNode<T: Tag> {
	pub title: String,
	pub chat: Node<ExampleChatLog, T>,
}

#[derive(Reflect, Default)]
pub struct ExampleChatLog {
	pub chats: Vec<String>,
}

rpc! {
	rpcs<T: Tag> ExampleNode<T> | ExampleRPC {
		rpc test_rpc(x: Node, message: String) {
			println!("hello from a client: {}", message);
		}

		rpc set_first(x: Node, msg: String) {
			x.as_mut().first = msg;
			x.member_modified("first".into());
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

	loop {
		thread::sleep(time::Duration::from_millis(100));
		client.update();

		println!("root.first = {}", client.as_ref().first);

		let mut buffer = String::new();
		stdin().read_line(&mut buffer).unwrap();

		client.set_first(buffer);

		println!("message sent.");
	}
}

pub fn server_main() {
	let mut server = Server::new();

	let listener = TcpListener::bind(ADDR).unwrap();

	listener.set_nonblocking(true).unwrap();

	println!("now listening on {}", ADDR);

	let mut next_frame = time::Instant::now();

	let mut missed_frames = 0;

	let node = server.make_node(ExampleNode {
		first: String::from("hoi"),
		second: 12,
		third: String::from("doei"),
	});

	loop {
		next_frame += time::Duration::from_millis(50);
		while time::Instant::now() >= next_frame {
			missed_frames += 1;
			println!("[WARN] Missed a frame! Total missed: {}", missed_frames);

			next_frame += time::Duration::from_millis(50);
		}

		listener.accept().map(|(stream, _)| {
			server.add_client(
				stream.try_clone().unwrap(), 
				stream.try_clone().unwrap(),
				node.clone());
		}).ok();

		server.update();

		thread::sleep(next_frame - time::Instant::now());		
	}
}