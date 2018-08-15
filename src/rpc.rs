use super::*;
use std::io::Cursor;

pub trait CallRPC {
    fn call_rpc(&mut self, msg: Deserializer<Cursor<Vec<u8>>, TagServer>);
}

#[macro_export]
macro_rules! rpc {
    // implement rpc functions for a type. These send a message to the server instead of executing the code.
    // The code is then executed on the server side using the CallRPC trait.
    ($($self:ty | $trait_name:ident {
        $(rpc $fn_name:ident(&mut self $(, $arg:ident : $arg_ty:ty)*) $body:expr)* 
    })*) => {$(
        pub trait $trait_name: NodeBase<TagClient> {
            $(
                // create an rpc requester function in the Node
                fn $fn_name(&mut self $(, mut $arg : $arg_ty)*) {
                    let mut ser = BufferSerializer { writer: Vec::new() };

                    String::from(stringify!($fn_name)).reflect(&mut ser);
                    $($arg.reflect(&mut ser);)*

                    self.send(ser);
                }
            )*
        }

        impl $trait_name for Node<$self, TagClient> { }

        impl CallRPC for $self {
            fn call_rpc(&mut self, mut msg: Deserializer<Cursor<Vec<u8>>, TagServer>) {
                let mut rpc_id = String::default();
                rpc_id.reflect(&mut msg);
                $(if rpc_id == stringify!($fn_name) {
                    // decode function arguments
                    $(let mut $arg: $arg_ty = Default::default(); $arg.reflect(&mut msg);)*
                    // evaluate function body
                    $body;
                } else)* {
                    panic!("requested rpc not found");
                }
            }
        })*
    };
}
