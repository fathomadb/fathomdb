//! Hidden-surface oracle fixture crate, moved variant: the same public paths
//! and effective gates as `../crate`, with items moved between files and
//! private modules and some `cfg`/`doc(hidden)` sites moved between a
//! definition and its re-export.
#![allow(dead_code)]

mod globbed;
mod inner;
mod nested {
    pub(crate) mod things;

    pub(crate) mod sealed_again {
        pub struct Token;
    }

    pub(crate) mod sealed {
        pub struct Token;
    }
}

pub use globbed::*;
#[cfg(debug_assertions)]
pub use inner::debug_reexported;
pub use inner::hook_for_test;
pub use inner::make;
pub use inner::make as make_alias;
pub use inner::plain_target;
#[cfg(feature = "hooks")]
pub use inner::reexport_gated_target;
pub use inner::{own_hidden_probe, use_hidden_probe, ImplProbe};
pub use inner::{BASE, DERIVED};
pub use nested::things::Probe;
pub use nested::things::{Shape, Thing};

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

/// Names two unreachable types that share kind and name.
pub fn takes_tokens(_: nested::sealed::Token, _: nested::sealed_again::Token) {}

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
