use std::cell::Cell;

thread_local! {
    static CALLBACK: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct Callback;
impl Callback {
    pub(crate) fn enter() -> Self {
        CALLBACK.with(|v| v.set(v.get() + 1));
        Self
    }
}
impl Drop for Callback {
    fn drop(&mut self) {
        CALLBACK.with(|v| v.set(v.get() - 1));
    }
}

pub(crate) fn in_callback() -> bool {
    CALLBACK.with(|v| v.get() != 0)
}
