//! Facade-shaped fixture crate: re-exports `hs_fixture` the way `fathomdb`
//! re-exports `fathomdb-engine`.

pub use hs_fixture::cyc::*;
#[cfg(debug_assertions)]
#[doc(hidden)]
pub use hs_fixture::debug_only_for_test;
#[doc(hidden)]
pub use hs_fixture::hidden_fn_for_test;
pub use hs_fixture::{make, Shape, Thing};

pub mod admin {
    pub use hs_fixture::sig_probe;
}

#[cfg(not(feature = "hooks"))]
#[doc(hidden)]
pub mod facade_absence_proof {}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
pub mod facade_release_proof {}
