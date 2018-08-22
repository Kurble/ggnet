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

/// Types that implement this trait can be reflected using the Visitor specified in V.
/// The implementor should call `Visitor::visit(..)` for named members,
///  while `Reflect::reflect(..)` will suffice for unnamed members.
pub trait Reflect<V: Visitor>: Default {
	/// Submits the data of `self` to the supplied visitor.
    fn reflect(&mut self, visitor: &mut V) -> Result<(), SerializeError>;
}

/// Defines a visitor that visits the contents of a type that implements `Reflect<..>`.
pub trait Visitor: Sized {
	/// Visit a named value. Should call `Reflect::reflect(..)` internally.
	/// If a visitor wants to do some kind of smart behaviour this function can be used to
	///  discriminate values based on their name.
    fn visit<T: Reflect<Self>>(&mut self, name: &str, val: &mut T) -> Result<(), SerializeError>;
}
