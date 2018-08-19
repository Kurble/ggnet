use super::*;
use std::io::Cursor;

pub trait CallRPC {
    fn call_rpc(node: &mut Any, msg: Deserializer<Cursor<Vec<u8>>>);
}

#[macro_export]
macro_rules! rpc {
    // implement rpc functions for a type. These send a message to the server instead of executing the code.
    // The code is then executed on the server side using the CallRPC trait.
    ($(rpcs<$($bound:ident : $bound_ty:path),*> $self:ty | $trait_name:ident {
        $(rpc $fn_name:ident($self_name:ident : Node $(, $arg:ident : $arg_ty:ty)*) $body:block)* 
    })*) => {$(
        // define a trait that enables the defined RPCs
        pub trait $trait_name {
            $(fn $fn_name(&mut self $(, $arg : $arg_ty)*);)*
        }

        // impl the trait to do requests
        impl<$($bound : $bound_ty),*> $trait_name for Node<$self, TagClient> where Self: NodeBase<TagClient> { 
            $(fn $fn_name(&mut self $(, mut $arg : $arg_ty)*) {
                let mut ser = BufferSerializer::new(vec![]);

                String::from(stringify!($fn_name)).reflect(&mut ser).unwrap();
                $($arg.reflect(&mut ser).unwrap();)*

                self.send(ser);
            })*
        }

        // impl the trait to execute functions
        impl<$($bound : $bound_ty),*> $trait_name for Node<$self, TagServer> where Self: NodeBase<TagServer> {
            $(fn $fn_name(&mut self $(, $arg : $arg_ty)*) {
                #[allow(unused)]
                let $self_name: &mut Self = self;
                $body
            })*
        }

        impl<$($bound : $bound_ty),*> CallRPC for $self {
            fn call_rpc(node: &mut Any, mut msg: Deserializer<Cursor<Vec<u8>>>) {
                let mut rpc_id = String::default();
                rpc_id.reflect(&mut msg).unwrap();
                $(if rpc_id == stringify!($fn_name) {
                    // decode function arguments
                    $(let mut $arg: $arg_ty = Default::default(); $arg.reflect(&mut msg).unwrap();)*
                    // evaluate function body
                    node.downcast_mut::<Node<$self, TagServer>>().unwrap().$fn_name($($arg),*);
                } else)* {
                    panic!("requested rpc not found");
                }
            }
        })*
    };
}
