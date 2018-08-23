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

pub use visitor::serializer::Serializer;
pub use visitor::deserializer::Deserializer;
pub use visitor::updater::NodeServerExt;
pub use node::{NodeBase, Node, Tag, TagServer, TagClient};
pub use rpc::*;
pub use connection::*;
pub use server::*;

/// Error type for ggnet related errors.
#[derive(Debug)]
pub enum Error {
    Custom(std::string::String),
    IOError(std::io::Error),
    UTFError(std::string::FromUtf8Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Error {
        Error::UTFError(err)
    }
}

/// Types that implement this trait can be reflected using the Visitor specified in V.
/// The implementor should call `Visitor::visit(..)` for named members,
///  while `Reflect::reflect(..)` will suffice for unnamed members.
pub trait Reflect<V: Visitor>: Default {
    /// Submits the data of `self` to the supplied visitor.
    fn reflect(&mut self, visitor: &mut V) -> Result<(), Error>;
}

/// Defines a visitor that visits the contents of a type that implements `Reflect<..>`.
pub trait Visitor: Sized {
    /// Visit a named value. Should call `Reflect::reflect(..)` internally.
    /// If a visitor wants to do some kind of smart behaviour this function can be used to
    ///  discriminate values based on their name.
    fn visit<T: Reflect<Self>>(&mut self, name: &str, val: &mut T) -> Result<(), Error>;
}
