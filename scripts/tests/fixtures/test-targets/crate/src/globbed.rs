pub fn g1() {}

pub struct G2;

/// Shadowed by the explicit root item of the same name.
pub fn shadowed() -> u16 {
    2
}
