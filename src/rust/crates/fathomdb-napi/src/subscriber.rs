//! Bounded, best-effort delivery of engine diagnostics to the Node event loop.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use fathomdb_engine::lifecycle::{
    Event, EventCategory, EventSource, Phase, ProfileRecord, SlowStatement, StressFailureContext,
    Subscriber,
};
use fathomdb_engine::Subscription;
use napi::{sys, Env, Error, JsFunction, NapiRaw, Result, Status};
use serde_json::{json, Value};

const CAPACITY: usize = 4096;
const DRAIN_BATCH: usize = 64;

#[derive(Default)]
struct Queue {
    records: VecDeque<Value>,
    wake_pending: bool,
}

pub(crate) struct Attachment {
    active: AtomicBool,
    dropped: AtomicU64,
    callback_faults: AtomicU64,
    queue: Mutex<Queue>,
    // The handle is represented as an address because N-API's opaque pointer
    // does not implement Send. The slot lock owns every call and final release.
    handle: Mutex<Option<usize>>,
    env: usize,
    cleanup_hook: Mutex<Option<usize>>,
}

struct Active {
    attachment: Arc<Attachment>,
    _subscription: Subscription,
}

#[derive(Default)]
pub(crate) struct SubscriberState {
    pub(crate) closing: bool,
    active: Option<Active>,
}

impl SubscriberState {
    pub(crate) fn replace(&mut self, attachment: Arc<Attachment>, subscription: Subscription) {
        if let Some(old) = self.active.take() {
            old.attachment.detach();
        }
        self.active = Some(Active { attachment, _subscription: subscription });
    }

    pub(crate) fn close(&mut self) {
        self.closing = true;
        if let Some(old) = self.active.take() {
            old.attachment.detach();
        }
    }
}

impl Attachment {
    pub(crate) fn new(env: Env, callback: JsFunction) -> Result<Arc<Self>> {
        let attachment = Arc::new(Self {
            active: AtomicBool::new(true),
            dropped: AtomicU64::new(0),
            callback_faults: AtomicU64::new(0),
            queue: Mutex::new(Queue::default()),
            handle: Mutex::new(None),
            env: env.raw() as usize,
            cleanup_hook: Mutex::new(None),
        });
        // N-API owns this Arc until its environment-thread finalizer. The
        // TSFN carries only null wake notifications, never boxed records.
        let context = Box::into_raw(Box::new(Arc::clone(&attachment)));
        let mut resource_name = ptr::null_mut();
        let name = b"fathomdb subscriber";
        let status = unsafe {
            sys::napi_create_string_utf8(
                env.raw(),
                name.as_ptr().cast(),
                name.len(),
                &mut resource_name,
            )
        };
        if status != sys::Status::napi_ok {
            unsafe { drop(Box::from_raw(context)) };
            return Err(Error::new(Status::GenericFailure, "subscriber resource name"));
        }
        let mut handle = ptr::null_mut();
        let status = unsafe {
            sys::napi_create_threadsafe_function(
                env.raw(),
                callback.raw(),
                ptr::null_mut(),
                resource_name,
                0,
                1,
                context.cast(),
                Some(finalize),
                context.cast(),
                Some(deliver),
                &mut handle,
            )
        };
        if status != sys::Status::napi_ok {
            unsafe { drop(Box::from_raw(context)) };
            return Err(Error::new(Status::GenericFailure, "subscriber wakeup creation"));
        }
        *attachment.handle.lock().unwrap_or_else(|poison| poison.into_inner()) =
            Some(handle as usize);
        let status = unsafe { sys::napi_unref_threadsafe_function(env.raw(), handle) };
        if status != sys::Status::napi_ok {
            attachment.detach();
            return Err(Error::new(Status::GenericFailure, "subscriber wakeup unref"));
        }
        // An unreferenced TSFN permits an idle environment to begin teardown.
        // Its initial thread count still needs releasing during that teardown.
        let hook = Arc::into_raw(Arc::clone(&attachment)) as usize;
        let status = unsafe {
            sys::napi_add_env_cleanup_hook(
                env.raw(),
                Some(environment_cleanup),
                hook as *mut c_void,
            )
        };
        if status != sys::Status::napi_ok {
            unsafe { drop(Arc::from_raw(hook as *const Attachment)) };
            attachment.detach();
            return Err(Error::new(Status::GenericFailure, "subscriber cleanup hook"));
        }
        *attachment.cleanup_hook.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(hook);
        Ok(attachment)
    }

    pub(crate) fn detach(&self) {
        if let Some(hook) =
            self.cleanup_hook.lock().unwrap_or_else(|poison| poison.into_inner()).take()
        {
            unsafe {
                let status = remove_cleanup_hook(self.env as sys::napi_env, hook);
                if status == sys::Status::napi_ok {
                    drop(Arc::from_raw(hook as *const Attachment));
                }
            }
        }
        self.detach_handle();
    }

    fn detach_handle(&self) {
        self.active.store(false, Ordering::Release);
        self.queue.lock().unwrap_or_else(|poison| poison.into_inner()).records.clear();
        let mut slot = self.handle.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(handle) = slot.take() {
            // A Rust unit-test executable has no Node N-API symbol table.
            // Test-hook Node artifacts use the real path (cfg(test) is false).
            #[cfg(not(test))]
            unsafe {
                sys::napi_release_threadsafe_function(
                    handle as sys::napi_threadsafe_function,
                    sys::ThreadsafeFunctionReleaseMode::abort,
                );
            }
            #[cfg(test)]
            let _ = handle;
        }
    }

    fn enqueue(&self, record: Value) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        // SQLite profile callbacks must never wait for a JS-thread lock.
        let Ok(slot) = self.handle.try_lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let Some(handle) = *slot else {
            return;
        };
        let Ok(mut queue) = self.queue.try_lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        if queue.records.len() == CAPACITY {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        queue.records.push_back(record);
        if !queue.wake_pending {
            queue.wake_pending = true;
            let status = unsafe { schedule_wakeup(handle) };
            if status != sys::Status::napi_ok {
                self.active.store(false, Ordering::Release);
                queue.records.clear();
                queue.wake_pending = false;
            }
        }
    }

    fn schedule_continuation(&self) {
        let slot = self.handle.lock().unwrap_or_else(|poison| poison.into_inner());
        let Some(handle) = *slot else {
            return;
        };
        let status = unsafe { schedule_wakeup(handle) };
        if status != sys::Status::napi_ok {
            self.active.store(false, Ordering::Release);
            let mut queue = self.queue.lock().unwrap_or_else(|poison| poison.into_inner());
            queue.records.clear();
            queue.wake_pending = false;
        }
    }

    fn drain(&self, env: sys::napi_env, callback: sys::napi_value) {
        for _ in 0..DRAIN_BATCH {
            if !self.active.load(Ordering::Acquire) {
                break;
            }
            let record = {
                let mut queue = self.queue.lock().unwrap_or_else(|poison| poison.into_inner());
                queue.records.pop_front()
            };
            let Some(mut record) = record else {
                break;
            };
            if !self.active.load(Ordering::Acquire) {
                break;
            }
            record["droppedRecordsTotal"] = json!(self.dropped.load(Ordering::Relaxed).to_string());
            let outcome =
                catch_unwind(AssertUnwindSafe(|| unsafe { call_listener(env, callback, &record) }));
            if !matches!(outcome, Ok(true)) {
                self.callback_faults.fetch_add(1, Ordering::Relaxed);
                unsafe { clear_exception(env) };
            }
        }
        let remaining = {
            let mut queue = self.queue.lock().unwrap_or_else(|poison| poison.into_inner());
            // Clearing and re-setting under one queue lock closes the gap
            // between a producer enqueue and continuation scheduling.
            queue.wake_pending = false;
            if self.active.load(Ordering::Acquire) && !queue.records.is_empty() {
                queue.wake_pending = true;
                true
            } else {
                false
            }
        };
        if remaining {
            self.schedule_continuation();
        }
    }
}

#[cfg(not(test))]
unsafe fn schedule_wakeup(handle: usize) -> sys::napi_status {
    sys::napi_call_threadsafe_function(
        handle as sys::napi_threadsafe_function,
        ptr::null_mut(),
        sys::ThreadsafeFunctionCallMode::nonblocking,
    )
}

#[cfg(test)]
unsafe fn schedule_wakeup(_handle: usize) -> sys::napi_status {
    sys::Status::napi_ok
}

#[cfg(not(test))]
unsafe fn remove_cleanup_hook(env: sys::napi_env, hook: usize) -> sys::napi_status {
    sys::napi_remove_env_cleanup_hook(env, Some(environment_cleanup), hook as *mut c_void)
}

#[cfg(test)]
unsafe fn remove_cleanup_hook(_env: sys::napi_env, _hook: usize) -> sys::napi_status {
    sys::Status::napi_ok
}

unsafe extern "C" fn environment_cleanup(context: *mut c_void) {
    if context.is_null() {
        return;
    }
    let attachment = Arc::from_raw(context as *const Attachment);
    attachment.cleanup_hook.lock().unwrap_or_else(|poison| poison.into_inner()).take();
    attachment.detach_handle();
}

unsafe fn call_listener(env: sys::napi_env, callback: sys::napi_value, record: &Value) -> bool {
    let js_env = Env::from_raw(env);
    let Ok(value) = js_env.to_js_value(record) else {
        return false;
    };
    let mut undefined = ptr::null_mut();
    if sys::napi_get_undefined(env, &mut undefined) != sys::Status::napi_ok {
        return false;
    }
    let argument = value.raw();
    let mut result = ptr::null_mut();
    sys::napi_call_function(env, undefined, callback, 1, &argument, &mut result)
        == sys::Status::napi_ok
}

unsafe fn clear_exception(env: sys::napi_env) {
    let mut pending = false;
    if sys::napi_is_exception_pending(env, &mut pending) == sys::Status::napi_ok && pending {
        let mut exception = ptr::null_mut();
        sys::napi_get_and_clear_last_exception(env, &mut exception);
    }
}

unsafe extern "C" fn deliver(
    env: sys::napi_env,
    callback: sys::napi_value,
    context: *mut c_void,
    _data: *mut c_void,
) {
    if env.is_null() || callback.is_null() || context.is_null() {
        return;
    }
    let attachment = &*(context as *const Arc<Attachment>);
    if catch_unwind(AssertUnwindSafe(|| attachment.drain(env, callback))).is_err() {
        attachment.callback_faults.fetch_add(1, Ordering::Relaxed);
        clear_exception(env);
    }
}

unsafe extern "C" fn finalize(_env: sys::napi_env, context: *mut c_void, _hint: *mut c_void) {
    if !context.is_null() {
        let attachment = Box::from_raw(context as *mut Arc<Attachment>);
        attachment.active.store(false, Ordering::Release);
        attachment.queue.lock().unwrap_or_else(|poison| poison.into_inner()).records.clear();
    }
}

impl Subscriber for Attachment {
    fn on_event(&self, event: &Event) {
        let phase = match event.phase {
            Phase::Started => "started",
            Phase::Slow => "slow",
            Phase::Heartbeat => "heartbeat",
            Phase::Finished => "finished",
            Phase::Failed => "failed",
        };
        let source = match event.source {
            EventSource::Engine => "engine",
            EventSource::SqliteInternal => "sqlite_internal",
        };
        let category = match event.category {
            EventCategory::Writer => "writer",
            EventCategory::Search => "search",
            EventCategory::Admin => "admin",
            EventCategory::Error => "error",
            EventCategory::Corruption => "corruption",
            EventCategory::Recovery => "recovery",
            EventCategory::Io => "io",
        };
        let mut record = json!({
            "kind": "event", "phase": phase, "source": source, "category": category,
        });
        if let Some(code) = event.code {
            record["code"] = json!(code);
        }
        self.enqueue(record);
    }

    fn on_profile(&self, record: &ProfileRecord) {
        self.enqueue(json!({
            "kind": "profile",
            "wallClockMs": record.wall_clock_ms.to_string(),
            "stepCount": record.step_count.to_string(),
            "cacheDelta": record.cache_delta.to_string(),
        }));
    }

    fn on_slow_statement(&self, signal: &SlowStatement) {
        self.enqueue(json!({
            "kind": "slowStatement",
            "statement": signal.statement,
            "wallClockMs": signal.wall_clock_ms.to_string(),
        }));
    }

    fn on_stress_failure(&self, context: &StressFailureContext) {
        self.enqueue(json!({
            "kind": "stressFailure",
            "threadGroupId": context.thread_group_id.to_string(),
            "opKind": context.op_kind,
            "lastErrorChain": context.last_error_chain,
            "projectionState": context.projection_state,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn unattached_for_queue_test() -> Arc<Attachment> {
        Arc::new(Attachment {
            active: AtomicBool::new(true),
            dropped: AtomicU64::new(0),
            callback_faults: AtomicU64::new(0),
            queue: Mutex::new(Queue::default()),
            handle: Mutex::new(Some(1)),
            env: 0,
            cleanup_hook: Mutex::new(None),
        })
    }

    fn event() -> Event {
        Event {
            phase: Phase::Started,
            source: EventSource::Engine,
            category: EventCategory::Writer,
            code: None,
        }
    }

    #[test]
    fn sqlite_producer_drops_without_waiting_for_a_busy_queue() {
        let attachment = unattached_for_queue_test();
        let held = attachment.queue.lock().unwrap();
        let (sent, received) = mpsc::channel();
        let producer = Arc::clone(&attachment);
        let thread = std::thread::spawn(move || {
            producer.on_event(&event());
            sent.send(()).unwrap();
        });
        received.recv_timeout(Duration::from_millis(200)).expect("producer must not block");
        drop(held);
        thread.join().unwrap();
        assert_eq!(attachment.dropped.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn full_record_queue_drops_new_record_at_4096() {
        let attachment = unattached_for_queue_test();
        {
            let mut queue = attachment.queue.lock().unwrap();
            queue.records.resize(CAPACITY, Value::Null);
        }
        attachment.on_event(&event());
        assert_eq!(attachment.queue.lock().unwrap().records.len(), CAPACITY);
        assert_eq!(attachment.dropped.load(Ordering::Relaxed), 1);
    }
}
