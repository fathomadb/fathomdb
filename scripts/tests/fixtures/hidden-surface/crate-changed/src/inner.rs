pub struct Thing {
    pub x: u32,
    #[doc(hidden)]
    pub hidden_field: u32,
    #[cfg(debug_assertions)]
    pub debug_field: u32,
    y: u32,
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

#[doc(hidden)]
impl Thing {
    pub fn hidden_impl_method(&self) {}
}

#[cfg(debug_assertions)]
impl Thing {
    pub fn debug_impl_method(&self) {}
}

impl Clone for Thing {
    fn clone(&self) -> Self {
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

impl From<Shape> for Thing {
    fn from(_: Shape) -> Self {
        make()
    }
}

impl crate::Probe for Thing {
    const LIMIT: u32 = 3;
    type Out = Shape;
    fn go(&self) -> Shape {
        Shape::Unit
    }
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

pub fn make() -> Thing {
    Thing {
        x: 1,
        hidden_field: 0,
        #[cfg(debug_assertions)]
        debug_field: 0,
        y: 2,
    }
}

#[doc(hidden)]
pub fn hook_for_test(thing: &Thing) -> Option<Thing> {
    let _ = thing;
    None
}

pub fn plain_target() {}

pub fn reexport_gated_target() {}

pub fn debug_reexported() {}

#[doc(hidden)]
pub fn own_hidden_probe() {}

pub fn use_hidden_probe() {}

pub struct ImplProbe;

#[doc(hidden)]
impl ImplProbe {
    pub fn method(&self) {}
}
