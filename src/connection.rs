use std::cmp::Ordering;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use super::*;

pub const PACKET_MAGIC: u32 = 0x12345678;

#[derive(Reflect, Default)]
pub struct Packet {
    pub node: u32,
    magic: u32,
    pub data: Vec<u8>,
}

struct Conn {
    w: Serializer<Box<Write>>,
    r: Receiver<Packet>,
}

pub struct Connection {
    inner: Arc<Mutex<Conn>>,
    id: usize,
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Connection { }

impl Ord for Connection {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl<T: Tag> NodeContext<T> for () {
    fn get(&self, _: u32) -> Option<Box<NodeBase<T>>> {
        unimplemented!();
    }

    fn insert(&mut self, _: u32, _: Box<NodeBase<T>>) {
        unimplemented!();
    }
}

impl Connection {
    pub fn new<W: 'static +  Write, R: 'static +  Read + Send>(w: W, r: R, id: usize) -> Self {
        let (sender, receiver) = channel();

        thread::spawn(move || {
            let mut de = Deserializer::<R, TagAgnostic>{ reader: r, context: Box::new(()) };
            loop {
                let mut packet = Packet::default();
                packet.reflect(&mut de);
                if packet.magic != PACKET_MAGIC {
                    break;
                }
                if sender.send(packet).is_err() {
                    break;
                }
            }
        });

        Connection{
            inner: Arc::new(Mutex::new(Conn { w: Serializer { writer: Box::new(w) }, r: receiver })),
            id
        }
    }

    pub fn send(&self, mut node: u32, data: &[u8]) {
        let mut conn = self.inner.lock().unwrap();

        node.reflect(&mut conn.w);
        PACKET_MAGIC.reflect(&mut conn.w);
        (data.len() as u32).reflect(&mut conn.w);
        conn.w.writer.write_all(data).unwrap();
    }

    pub fn recv(&self) -> Option<Packet> {
        let conn = self.inner.lock().unwrap();
        conn.r.try_recv().ok()
    }

    pub fn recv_blocking(&self) -> Packet {
        let conn = self.inner.lock().unwrap();
        conn.r.recv().unwrap()
    }
}
