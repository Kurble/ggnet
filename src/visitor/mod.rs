use super::*;

pub mod encode;
pub mod serializer;
pub mod deserializer;
pub mod updater;
pub mod printer;
pub mod refresher;

macro_rules! reflect_tuple {
    ($(($($x:ident),*))*) => {$(
        impl<V, $($x),*> Reflect<V> for ($($x),*) where
            V: Visitor,
            $($x: Reflect<V> + Default, )*
        {
            fn reflect(&mut self, visit: &mut V) -> Result<(), Error> {
                #[allow(non_snake_case)]
                let ($(ref mut $x),*) = self;
                $($x.reflect(visit)?;)*
                Ok(())
            }
        }
    )*}
}

reflect_tuple!{
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
    (A, B, C, D, E, F, G)
    (A, B, C, D, E, F, G, H)
    (A, B, C, D, E, F, G, H, I)
    (A, B, C, D, E, F, G, H, I, J)
    (A, B, C, D, E, F, G, H, I, J, K)
    (A, B, C, D, E, F, G, H, I, J, K, L)
}

impl<V, T> Reflect<V> for Option<T> where
    V: Visitor,
    T: Reflect<V>,
    bool: Reflect<V>,
{
    fn reflect(&mut self, visit: &mut V) -> Result<(), Error> {
        let mut is_some = self.is_some();
        let was_some = is_some;
        is_some.reflect(visit)?;

        // set appropriate variant
        if was_some != is_some {
            *self = if is_some {
                Some(Default::default())
            } else {
                None
            }
        }

        // reflect variant
        if is_some {
            self.as_mut().unwrap().reflect(visit)?;
        }

        Ok(())
    }
}

use std::marker::PhantomData;
impl<V, T> Reflect<V> for PhantomData<T> where
    V: Visitor,
{
    fn reflect(&mut self, _: &mut V) -> Result<(), Error> { Ok(()) }
}

use std::time::*;

impl<V> Reflect<V> for Duration where
    V: Visitor,
    u64: Reflect<V>,
    u32: Reflect<V>,
{
    fn reflect(&mut self, visit: &mut V) -> Result<(), Error> {
        let mut secs = self.as_secs();
        secs.reflect(visit)?;
        let mut nanos = self.subsec_nanos();
        nanos.reflect(visit)?;
        *self = Duration::new(secs, nanos);  
        Ok(())      
    }
}