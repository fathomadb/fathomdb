use crate::nested::things::Thing;

pub const BASE: u32 = 7;
pub const DERIVED: u32 = BASE * 2;

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

pub fn own_hidden_probe() {}

pub fn use_hidden_probe() {}

pub struct ImplProbe;

impl ImplProbe {
    pub fn method(&self) {}
}
