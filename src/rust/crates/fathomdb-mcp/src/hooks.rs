//! The seam points the bridge and edge call. Without the `test-seams`
//! feature every hook is an inlined no-op.

#[derive(Clone, Default)]
pub(crate) struct Hooks {
    #[cfg(feature = "test-seams")]
    pub(crate) seams: crate::seams::Seams,
}

impl Hooks {
    #[inline]
    pub(crate) fn before_open(&self, _store: &str) {
        #[cfg(feature = "test-seams")]
        self.seams.before_open(_store);
    }

    #[inline]
    pub(crate) fn before_call(&self, _store: &str) {
        #[cfg(feature = "test-seams")]
        self.seams.before_call(_store);
    }

    #[inline]
    pub(crate) fn tool_panics(&self, _tool: &str) -> bool {
        #[cfg(feature = "test-seams")]
        return self.seams.tool_panics(_tool);
        #[cfg(not(feature = "test-seams"))]
        false
    }
}
