use super::*;
use std::sync::Condvar;

#[derive(Default)]
pub(crate) struct PhysicalReaderAdmission {
    state: Mutex<(bool, usize)>,
    changed: Condvar,
}

pub(crate) struct PhysicalRead<'a>(&'a PhysicalReaderAdmission);
pub(crate) struct PhysicalReaderPause<'a>(&'a PhysicalReaderAdmission);

impl PhysicalReaderAdmission {
    pub(crate) fn enter(&self) -> PhysicalRead<'_> {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut state =
            self.changed.wait_while(state, |state| state.0).unwrap_or_else(|p| p.into_inner());
        state.1 += 1;
        PhysicalRead(self)
    }

    pub(crate) fn pause(&self) -> Result<PhysicalReaderPause<'_>, EngineError> {
        let mut state = self.state.lock().map_err(|_| EngineError::Storage)?;
        state.0 = true;
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, Duration::from_millis(100), |state| state.1 != 0)
            .map_err(|_| EngineError::Storage)?;
        if state.1 != 0 {
            state.0 = false;
            self.changed.notify_all();
            return Err(EngineError::ErasureIncomplete {
                stage: "wal_checkpoint".into(),
                detail: "owned readers remain active after the finite cleanup drain attempt".into(),
            });
        }
        Ok(PhysicalReaderPause(self))
    }
}

impl Drop for PhysicalRead<'_> {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap_or_else(|p| p.into_inner());
        state.1 -= 1;
        self.0.changed.notify_all();
    }
}

impl Drop for PhysicalReaderPause<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(|p| p.into_inner()).0 = false;
        self.0.changed.notify_all();
    }
}
