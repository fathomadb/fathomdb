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
    pub(crate) fn after_queued(&self, _store: &str) {
        #[cfg(feature = "test-seams")]
        self.seams.after_queued(_store);
    }

    #[inline]
    pub(crate) async fn after_permit(&self, _store: &str) {
        #[cfg(feature = "test-seams")]
        self.seams.after_permit(_store).await;
    }

    #[inline]
    pub(crate) fn tool_panics(&self, _tool: &str) -> bool {
        #[cfg(feature = "test-seams")]
        return self.seams.tool_panics(_tool);
        #[cfg(not(feature = "test-seams"))]
        false
    }

    #[inline]
    pub(crate) fn progress_burst(&self, _tool: &str) -> u32 {
        #[cfg(feature = "test-seams")]
        return self.seams.progress_burst(_tool);
        #[cfg(not(feature = "test-seams"))]
        0
    }
}
