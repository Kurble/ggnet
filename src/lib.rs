extern crate byteorder;
#[macro_use] extern crate ggnet_derive;

mod encode;
mod node;
mod connection;
mod serializer;
mod deserializer;
mod updater;
mod rpc;
mod printer;
mod server;

use std::collections::HashMap;
use std::any::Any;
use std::hash::Hash;
use std::sync::{Arc,Mutex};
use std::io::{Read,Write};

use self::encode::{encode, decode};
pub use self::serializer::Serializer;
pub use self::deserializer::Deserializer;
pub use self::updater::NodeServerExt;

pub use node::*;
pub use rpc::*;
pub use connection::*;
pub use server::*;

pub trait Reflect<V: Visitor>: Default {
    fn reflect(&mut self, &mut V);
}

pub trait Visitor: Sized {
    fn visit<T: Reflect<Self>>(&mut self, name: &str, val: &mut T);
}
