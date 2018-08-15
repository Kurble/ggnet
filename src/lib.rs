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

use encode::{encode, decode};
pub use encode::SerializeError;
pub use serializer::Serializer;
pub use deserializer::Deserializer;
pub use updater::NodeServerExt;
pub use node::*;
pub use rpc::*;
pub use connection::*;
pub use server::*;

pub trait Reflect<V: Visitor>: Default {
    fn reflect(&mut self, &mut V) -> Result<(), SerializeError>;
}

pub trait Visitor: Sized {
    fn visit<T: Reflect<Self>>(&mut self, name: &str, val: &mut T) -> Result<(), SerializeError>;
}
