use crate::inner::make;

pub trait Probe {
    const LIMIT: u32;
    type Out;
    fn go(&self) -> Self::Out;
    fn provided(&self) {}
}

pub enum Shape {
    Unit,
    Pair(u32, Thing),
    Named {
        a: u32,
        #[doc(hidden)]
        b: u8,
    },
}

pub struct Thing {
    pub x: u32,
    #[doc(hidden)]
    pub hidden_field: u32,
    #[cfg(debug_assertions)]
    pub debug_field: u32,
    pub(crate) y: u32,
}

impl Probe for Thing {
    const LIMIT: u32 = 3;
    type Out = Shape;
    fn go(&self) -> Shape {
        Shape::Unit
    }
}

impl From<Shape> for Thing {
    fn from(_: Shape) -> Self {
        make()
    }
}

impl From<u32> for Thing {
    fn from(x: u32) -> Self {
        let mut thing = make();
        thing.x = x;
        thing
    }
}

impl Clone for Thing {
    fn clone(&self) -> Self {
        make()
    }
}

#[cfg(debug_assertions)]
impl Thing {
    pub fn debug_impl_method(&self) {}
}

#[doc(hidden)]
impl Thing {
    pub fn hidden_impl_method(&self) {}
}

impl Thing {
    pub fn get(&self) -> u32 {
        self.x + self.y
    }

    #[doc(hidden)]
    pub fn peek_for_test(&self) -> u32 {
        self.y
    }
}
