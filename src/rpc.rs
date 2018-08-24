use super::*;
use std::io::Cursor;

/// Implemented by the `rpc!` macro. This trait performs a remote procedure call (RPC) that is 
///  encoded in the supplied message.
/// Do not manually implement this trait, use `rpc!`.
pub trait CallRPC {
    /// Process rpc call encoded in the `message` parameter. 
    /// The `node` parameter is expected to be a mutable reference in the form `&mut Node<T, TagServer>`.
    fn call_rpc(node: &mut Any, message: Deserializer<Cursor<Vec<u8>>>);
}

/// Implement the `CallRPC` trait for any number of types. 
/// The macro creates a trait with the defined functions for each type that should be named uniquely.
/// * `Node<T, TagClient>` will implement this trait by sending RPC requests.
/// * `Node<T, TagServer>` will implement this trait by performing the functions.
///
/// ! #[macro_use] extern crate ggnet;
/// ! #[macro_use] extern crate ggnet_derive;
/// ! 
/// ! // RPC types must implement `Reflect<V>` and `Default`.
/// ! #[derive(Reflect, Default)]
/// ! pub struct Foo {
/// !     foo: String,
/// !     bar: u32,
/// ! }
/// ! 
/// ! rpc! {
/// !     // A unique trait name for the defined rpc's must be provided. Here `FooRPC` is used.
/// !     rpcs< /* no type arguments */ > FooRPC for Foo {
/// !         // because `self` is not available as an identifier in macros, an alternative
/// !         //  name for self must be provided. This should be the first rpc parameter and have 
/// !         //  the type `Node`. It behaves as if it were `&mut Node<Foo, TagServer>`.
/// !         // RPC's are not allowed to have a return type.
/// !         rpc hello_foo(x: Node, greeting: String) {
/// !             // print a message on the server
/// !             println!("Hello from client: {}", greeting);
/// !             // modify the node
/// !             x.as_mut().foo = greeting;
/// !             // notify subscribed clients of this change
/// !             x.member_modified("foo".into());
/// !         }
/// !     }
/// ! }
/// .

#[macro_export]
macro_rules! rpc {
    // implement rpc functions for a type. 
    // These send a message to the server instead of executing the code.
    // The code is then executed on the server side using the CallRPC trait.
    ($(rpcs<$($bound:ident : $bound_ty:path),*> $trait_name:ident for $self:ty  {
        $(rpc $fn_name:ident($self_name:ident : Node $(, $arg:ident : $arg_ty:ty)*) $body:block)* 
    })*) => {$(
        // define a trait that enables the defined RPCs
        pub trait $trait_name {
            $(fn $fn_name(&mut self $(, $arg : $arg_ty)*);)*
        }

        // impl the trait to do requests
        impl<$($bound : $bound_ty),*> $trait_name for Node<$self, TagClient> where 
            Self: NodeBase<TagClient> 
        { 
            $(fn $fn_name(&mut self $(, mut $arg : $arg_ty)*) {
                let mut ser = Serializer::new(vec![]);

                String::from(stringify!($fn_name)).reflect(&mut ser).unwrap();
                $($arg.reflect(&mut ser).unwrap();)*

                self.send(ser);
            })*
        }

        // impl the trait to execute functions
        impl<$($bound : $bound_ty),*> $trait_name for Node<$self, TagServer> where 
            Self: NodeBase<TagServer> 
        {
            $(fn $fn_name(&mut self $(, $arg : $arg_ty)*) {
                #[allow(unused)]
                let $self_name: &mut Self = self;
                $body
            })*
        }

        impl<$($bound : $bound_ty),*> CallRPC for $self {
            fn call_rpc(node: &mut ::std::any::Any, mut msg: Deserializer<::std::io::Cursor<::std::vec::Vec<u8>>>) {
                let mut rpc_id = String::default();
                rpc_id.reflect(&mut msg).unwrap();
                $(if rpc_id == stringify!($fn_name) {
                    // decode function arguments
                    $(let mut $arg: $arg_ty = ::std::default::Default::default(); $arg.reflect(&mut msg).unwrap();)*
                    // evaluate function body
                    node.downcast_mut::<Node<$self, TagServer>>().unwrap().$fn_name($($arg),*);
                } else)* {
                    panic!("requested rpc not found");
                }
            }
        })*
    };
}
