use super::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Condvar;

thread_local! {
    static OPERATIONS: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
    static CALLBACK: Cell<usize> = const { Cell::new(0) };
    static PERMIT: Cell<Option<(usize, u64)>> = const { Cell::new(None) };
}

#[derive(Default)]
struct State {
    closing: bool,
    acquiring: bool,
    owner: Option<u64>,
    generation: u64,
    active: usize,
}

#[derive(Default)]
pub(crate) struct Admission {
    state: Mutex<State>,
    changed: Condvar,
}

pub(crate) struct Operation {
    admission: Arc<Admission>,
    key: usize,
    _same_thread: PhantomData<Rc<()>>,
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

impl Admission {
    fn key(&self) -> usize {
        self as *const Self as usize
    }

    pub(crate) fn enter(self: &Arc<Self>) -> Result<Operation, EngineError> {
        let key = self.key();
        let mut s = self.state.lock().map_err(|_| EngineError::Storage)?;
        if s.closing {
            return Err(EngineError::Closing);
        }
        let nested =
            !in_callback() && OPERATIONS.with(|v| v.borrow().get(&key).copied().unwrap_or(0) > 0);
        let permitted = !in_callback()
            && PERMIT.with(|v| v.get().is_some_and(|(k, t)| k == key && s.owner == Some(t)));
        if (s.acquiring || s.owner.is_some()) && !nested && !permitted {
            return Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy));
        }
        s.active += 1;
        OPERATIONS.with(|v| *v.borrow_mut().entry(key).or_default() += 1);
        Ok(Operation { admission: Arc::clone(self), key, _same_thread: PhantomData })
    }

    pub(crate) fn acquire(&self, timeout: Duration) -> Result<u64, EngineError> {
        if in_callback()
            || OPERATIONS.with(|v| v.borrow().get(&self.key()).copied().unwrap_or(0) != 0)
        {
            return Err(EngineError::Maintenance(MaintenanceError::SelfPinnedReader));
        }
        let mut s = self.state.lock().map_err(|_| EngineError::Storage)?;
        if s.closing {
            return Err(EngineError::Closing);
        }
        if s.acquiring || s.owner.is_some() {
            return Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy));
        }
        s.acquiring = true;
        let (mut s, _) = self
            .changed
            .wait_timeout_while(s, timeout, |s| s.active != 0 && !s.closing)
            .map_err(|_| EngineError::Storage)?;
        if s.closing || s.active != 0 {
            s.acquiring = false;
            self.changed.notify_all();
            return Err(if s.closing {
                EngineError::Closing
            } else {
                EngineError::Maintenance(MaintenanceError::MaintenanceBusy)
            });
        }
        s.generation = s.generation.checked_add(1).ok_or(EngineError::Storage)?;
        let token = s.generation;
        s.owner = Some(token);
        s.acquiring = false;
        Ok(token)
    }

    pub(crate) fn release(&self, token: u64) -> bool {
        let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if s.owner == Some(token) {
            s.owner = None;
            self.changed.notify_all();
            true
        } else {
            false
        }
    }

    pub(crate) fn valid(&self, token: u64) -> Result<(), EngineError> {
        let s = self.state.lock().map_err(|_| EngineError::Storage)?;
        if s.closing {
            return Err(EngineError::Closing);
        }
        if s.owner != Some(token) {
            return Err(EngineError::Maintenance(MaintenanceError::StaleHandle));
        }
        Ok(())
    }

    pub(crate) fn with_permit<T>(
        &self,
        token: u64,
        operation: impl FnOnce() -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        self.valid(token)?;
        struct Restore(Option<(usize, u64)>);
        impl Drop for Restore {
            fn drop(&mut self) {
                PERMIT.with(|v| v.set(self.0));
            }
        }
        let _restore = Restore(PERMIT.with(|v| v.replace(Some((self.key(), token)))));
        connection_runtime::internal_debt_probe(operation)
    }

    pub(crate) fn begin_close(&self) -> Result<(), EngineError> {
        if in_callback()
            || OPERATIONS.with(|v| v.borrow().get(&self.key()).copied().unwrap_or(0) != 0)
        {
            return Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy));
        }
        let mut s = self.state.lock().map_err(|_| EngineError::Storage)?;
        s.closing = true;
        s.owner = None;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn wait_for_quiescence(&self) -> Result<(), EngineError> {
        let state = self.state.lock().map_err(|_| EngineError::Storage)?;
        drop(
            self.changed
                .wait_while(state, |state| state.active != 0)
                .map_err(|_| EngineError::Storage)?,
        );
        Ok(())
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        OPERATIONS.with(|v| {
            let mut values = v.borrow_mut();
            if let Some(depth) = values.get_mut(&self.key) {
                *depth -= 1;
                if *depth == 0 {
                    values.remove(&self.key);
                }
            }
        });
        let mut s = self.admission.state.lock().unwrap_or_else(|p| p.into_inner());
        s.active -= 1;
        self.admission.changed.notify_all();
    }
}
