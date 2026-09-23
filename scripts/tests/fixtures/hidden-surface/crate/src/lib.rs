//! Hidden-surface oracle fixture crate. Every item exists to exercise one
//! reachability, normalization, or cfg case in `dev/tools/hidden_surface.py`.
#![allow(dead_code)]

mod globbed;
mod inner;

pub use globbed::*;
#[cfg(debug_assertions)]
#[doc(hidden)]
pub use inner::debug_reexported;
pub use inner::hook_for_test;
pub use inner::make as make_alias;
#[doc(hidden)]
pub use inner::plain_target;
#[cfg(feature = "hooks")]
pub use inner::reexport_gated_target;
pub use inner::{make, Shape, Thing};
pub use inner::{own_hidden_probe, use_hidden_probe, ImplProbe};

/// Explicit item that shadows the glob-imported `shadowed` from `globbed`.
pub fn shadowed() -> u8 {
    1
}

#[doc(hidden)]
pub fn hidden_fn_for_test() {}

#[doc(hidden)]
pub mod hidden_mod {
    pub fn inside() {}
}

pub mod visible_mod {
    pub fn vis() {}
}

pub use visible_mod as vm_alias;

/// Cyclic re-export: `cyc::again` names `cyc` itself.
pub mod cyc {
    pub use crate::cyc as again;
    pub fn in_cyc() {}
}

/// External-crate glob re-export.
pub mod ext_glob {
    pub use core::hint::*;
}

#[cfg(feature = "hooks")]
#[doc(hidden)]
pub fn gated_hook() {}

#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn debug_only_for_test() {}

#[cfg(not(feature = "hooks"))]
pub fn without_hooks() {}

#[cfg(any(debug_assertions, feature = "hooks", test))]
#[doc(hidden)]
pub fn any_gate() {}

#[cfg(all(debug_assertions, not(feature = "hooks")))]
pub fn all_gate() {}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
pub mod release_only_proof {}

pub const BASE: u32 = 7;
pub const DERIVED: u32 = BASE * 2;

pub trait Probe {
    const LIMIT: u32;
    type Out;
    fn go(&self) -> Self::Out;
    fn provided(&self) {}
}

pub trait Marker {}

/// Local blanket impl.
impl<T: Probe> Marker for T {}

pub trait DebugOnlyTrait {}

#[cfg(debug_assertions)]
impl DebugOnlyTrait for Thing {}

pub struct KindProbe;

pub fn sig_probe(x: u32) -> u32 {
    x
}

pub fn vis_probe() {}

#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn cfg_probe() {}

pub struct Tuple(pub u32, #[doc(hidden)] pub u8, u16);

mod sealed {
    pub struct Token;
}

mod sealed_again {
    pub struct Token;
}

/// Names two unreachable types that share kind and name.
pub fn takes_tokens(_: sealed::Token, _: sealed_again::Token) {}

#[cfg(debug_assertions)]
pub use inner::gate_both_probe;

#[cfg(debug_assertions)]
pub fn cfg_drop_probe() {}

#[cfg(any(debug_assertions, feature = "hooks"))]
pub fn cfg_narrow_probe() {}

#[cfg(test)]
mod tests {
    #[test]
    fn unit_in_lib() {}
}
