use super::*;
use std::cmp::Ordering;
use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicBool,Ordering as AtomicOrdering};
use std::thread;
use std::hash::{Hash, Hasher};

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

/// A `Connection`. This struct wraps around a `Write` and `Read` implementation that should be
///  mapped to a tcp socket.
pub struct Connection {
    inner: Arc<Mutex<Conn>>,
    err: Arc<Mutex<Option<Error>>>,
    alive: Arc<AtomicBool>,
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

impl Hash for Connection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
            err: self.err.clone(),
            alive: self.alive.clone(),
            id: self.id,
        }
    }
}

impl Connection {
    /// Initialize a new `Connection`. 
    /// When used for a `Client<T>` the `id` parameter can be anything, it is only used in `Server`.
    /// The id is what determines ordering and equality for `Connection`.
    pub fn new<W: 'static +  Write, R: 'static +  Read + Send>(w: W, r: R, id: usize) -> Self {
        let (sender, receiver) = channel();

        let inner = Arc::new(Mutex::new(Conn { w: Serializer::new(Box::new(w)), r: receiver }));
        let alive = Arc::new(AtomicBool::new(true));
        let err = Arc::new(Mutex::new(None));

        let result = Connection{
            inner: inner.clone(),
            alive: alive.clone(),
            err: err.clone(),
            id
        };

        thread::spawn(move || {
            let mut de = Deserializer::new(r);
            while alive.load(AtomicOrdering::Relaxed) {
                let mut packet = Packet::default();
                let result = packet.reflect(&mut de);
                if result.is_err() {
                    *err.lock().unwrap() = Some(result.err().unwrap());
                    break;
                }
                if packet.magic != PACKET_MAGIC {
                    *err.lock().unwrap() = Some(Error::Custom("Corrupt Packet".into()));
                    break;
                }
                if sender.send(packet).is_err() {
                    *err.lock().unwrap() = Some(Error::Custom("Channel Error".into()));
                    break;
                }
            }

            alive.swap(false, AtomicOrdering::Relaxed);
        });  

        result     
    }

    /// When one of the wrapped `Write` or `Read` implementations return an error
    ///  the `Connection` is flagged as dead internally. This function can be used to check if the
    /// `Connection` is still alive. 
    /// This function will return `Ok(())` if the `Connection` is alive, or an `Err(_)` if it isn't.
    pub fn status(&self) -> Result<(), Error> {
        if self.alive.load(AtomicOrdering::Relaxed) {
            Ok(())
        } else {
            Err(self.err.lock().unwrap().take().unwrap())
        }
    }

    /// Send a message destined for the `Node` with id `node` over the `Connection`.
    pub fn send(&self, mut node: u32, data: &[u8]) {
        let mut conn = self.inner.lock().unwrap();

        let mut x = move || -> Result<(), Error> {
            node.reflect(&mut conn.w)?;
            PACKET_MAGIC.reflect(&mut conn.w)?;
            (data.len() as u32).reflect(&mut conn.w)?;
            conn.w.writer.write_all(data)?;

            Ok(())
        };

        let result = x();
        if result.is_err() {
            *self.err.lock().unwrap() = result.err();
            self.alive.swap(false, AtomicOrdering::Relaxed);
        }
    }

    /// Returns `Some(Packet)` if there is one available now, otherwise returns `None`.
    pub fn recv(&self) -> Option<Packet> {
        let conn = self.inner.lock().unwrap();
        conn.r.try_recv().ok()
    }

    /// Blocks until a `Packet` is available and then returns `Some(Packet)`. 
    /// If the `Connection` dies while blocking, this function will return `None`.
    pub fn recv_blocking(&self) -> Option<Packet> {
        let conn = self.inner.lock().unwrap();
        conn.r.recv().ok()
    }
}
