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
    rpcs<T: Tag> ExampleRPC for ExampleNode<T> {
        rpc hello(x: Node, message: String) {
            println!("client connected: {}", message);
        }

        rpc set_title(x: Node, title: String) {
            x.as_mut().title = title;
            x.member_modified("title".into());
        }
    }

    rpcs<> ChatRPC for ExampleChatLog {
        rpc chat(x: Node, msg: String) {
            x.member_vec_push("chats".into(), msg);
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

    let mut server = Client::<ExampleNode<TagClient>>::new(
        Connection::new(stream.try_clone().unwrap(), stream.try_clone().unwrap(),0)
    );

    server.hello("brams client".into());

    loop {
        thread::sleep(time::Duration::from_millis(100));
        server.update();

        println!("Chat room: {}", server.as_ref().title);
        println!("------------------------------------");
        {
            let server = server.as_ref();
            for msg in server.chat.as_ref().chats.iter() {
                println!("{}", msg);
            }
        }

        let mut buffer = String::new();
        stdin().read_line(&mut buffer).unwrap();

        server.as_mut().chat.chat(buffer);
    }
}

pub fn server_main() {
    let mut server = Server::new();

    let listener = TcpListener::bind(ADDR).unwrap();

    listener.set_nonblocking(true).unwrap();

    println!("now listening on {}", ADDR);

    let mut next_frame = time::Instant::now();

    let mut missed_frames = 0;

    let chat = server.make_node(ExampleChatLog {
        chats: vec![],
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
                ExampleNode {
                    title: String::from("Example Server"),
                    chat: chat.clone(),
                });
        }).ok();

        server.update();

        thread::sleep(next_frame - time::Instant::now());       
    }
}