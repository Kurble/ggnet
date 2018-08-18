extern crate byteorder;
#[macro_use] extern crate ggnet_derive;

mod visitor;
mod node;
mod connection;
mod rpc;
mod server;

use std::collections::HashMap;
use std::any::Any;
use std::hash::Hash;
use std::sync::{Arc,Mutex};
use std::io::{Read,Write};

use visitor::encode::{encode, decode};
pub use visitor::encode::SerializeError;
pub use visitor::serializer::Serializer;
pub use visitor::deserializer::Deserializer;
pub use visitor::updater::NodeServerExt;

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
