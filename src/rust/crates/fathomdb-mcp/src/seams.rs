//! Adapter-only test seams (feature `test-seams`).

/// Injectable delays and faults.
#[derive(Clone, Default)]
pub struct Seams;

impl Seams {
    /// Make the next calls to `tool` panic inside the handler.
    pub fn panic_in_tool(&self, _tool: &str) {
        todo!()
    }
}
