use super::*;

pub mod encode;
pub mod serializer;
pub mod deserializer;
pub mod updater;
pub mod printer;
pub mod refresher;

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
