use super::*;

pub struct Refresher {
    context: Box<Any>,
}

impl Refresher {
    pub fn new<G: Tag>(context: Arc<Mutex<NodeContext<G>>>) -> Self {
        Self {
            context: Box::new(context),
        }
    }

    pub fn context<G: Tag>(&self) -> Arc<Mutex<NodeContext<G>>> {
        self.context.downcast_ref::<Arc<Mutex<NodeContext<G>>>>().unwrap().clone()
    }
}

impl Visitor for Refresher {
    fn visit<T: Reflect<Refresher>>(&mut self, _: &str, val: &mut T) -> Result<(), SerializeError> {
        Ok(val.reflect(self)?)
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl Reflect<Refresher> for $t {
            fn reflect(&mut self, _: &mut Refresher) -> Result<(), SerializeError> {
                Ok(())
            }
        }
    )
}

encodable!{ u16 }
encodable!{ i16 }
encodable!{ u32 }
encodable!{ i32 }
encodable!{ f32 }
encodable!{ u64 }
encodable!{ i64 }
encodable!{ f64 }
encodable!{ bool }
encodable!{ String }

impl<T: Reflect<Refresher>> Reflect<Refresher> for Vec<T> {
    fn reflect(&mut self, visit: &mut Refresher) -> Result<(), SerializeError> {
        for e in self.iter_mut() {
            e.reflect(visit)?;
        }
        Ok(())
    }
}

impl<K: Reflect<Refresher> + Eq + Hash, V: Reflect<Refresher>> Reflect<Refresher> for HashMap<K,V> {
    fn reflect(&mut self, visit: &mut Refresher) -> Result<(), SerializeError> {
        for (_,v) in self.iter_mut() {
            v.reflect(visit)?;
        }
        Ok(())
    }
}
