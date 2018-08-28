use super::*;

pub struct Printer {
    pub result: String,
    pub indent: String,
}

impl Visitor for Printer {
    fn visit<T: Reflect<Printer>>(&mut self, name: &str, val: &mut T) -> Result<(), Error> {
        self.indent.push_str("  ");
        self.result.push_str(&format!("{} {{\n{}", name, self.indent));
        val.reflect(self)?;
        self.indent.pop();
        self.indent.pop();
        self.result.push_str(&format!("\n{}}}\n{}", self.indent, self.indent));
        Ok(())
    }
}

macro_rules! encodable {
    ($t:ty) => (
        impl Reflect<Printer> for $t {
            fn reflect(&mut self, visit: &mut Printer) -> Result<(), Error> {
                visit.result.push_str(&format!("{}, ", *self));
                Ok(())
            }
        }
    )
}

encodable!{ u8 }
encodable!{ i8 }
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

impl<T: Reflect<Printer>> Reflect<Printer> for Vec<T> {
    fn reflect(&mut self, visit: &mut Printer) -> Result<(), Error> {
        visit.result.push_str("vec![ ");
        for e in self.iter_mut() {
            e.reflect(visit)?;
            visit.result.push_str(",");
        }
        visit.result.push_str(" ]");
        Ok(())
    }
}

impl<K: Reflect<Printer> + Eq + Hash + Clone, V: Reflect<Printer>> Reflect<Printer> for HashMap<K, V> {
    fn reflect(&mut self, visit: &mut Printer) -> Result<(), Error> {
        visit.result.push_str("map![ ");
        for (k, v) in self.iter_mut() {
            visit.result.push_str("(");
            let mut kc: K = k.clone();
            kc.reflect(visit)?;
            v.reflect(visit)?;
            visit.result.push_str("),");
        }
        visit.result.push_str(" ]");
        Ok(())
    }
}