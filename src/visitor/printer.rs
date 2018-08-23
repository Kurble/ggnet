use super::*;

pub struct Printer {
    pub result: String,
    pub indent: String,
}

impl Visitor for Printer {
    fn visit<T: Reflect<Printer>>(&mut self, name: &str, val: &mut T) -> Result<(), Error> {
        self.indent.push_str("\t");
        self.result.push_str(&format!("{} {{\n{}", name, self.indent));
        val.reflect(self)?;
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
        (self.len() as u32).reflect(visit)?;
        for e in self.iter_mut() {
            e.reflect(visit)?;
        }
        Ok(())
    }
}
