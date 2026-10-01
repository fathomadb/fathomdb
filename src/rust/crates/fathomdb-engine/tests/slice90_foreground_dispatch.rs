//! Foreground provider calls share the engine's bounded inference deadline.

use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, EngineError};

#[derive(Debug)]
struct ParkedEmbedder {
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for ParkedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("foreground-parked", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let _ = self.release.lock().expect("release lock").recv_timeout(Duration::from_millis(500));
        Ok(vec![1.0; 8])
    }
}

#[test]
fn direct_embed_started_timeout_uses_engine_dispatch_deadline() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("direct-timeout.sqlite");
    let (release, held) = mpsc::channel();
    let provider = Arc::new(ParkedEmbedder { release: Mutex::new(held) });
    let opened = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    )
    .expect("open");
    let started = Instant::now();
    let result = opened.engine.embed_text("deadline probe");
    let elapsed = started.elapsed();
    release.send(()).ok();
    assert!(matches!(result, Err(EngineError::Embedder)), "started timeout is an embedder error");
    assert!(elapsed < Duration::from_millis(250), "direct call must honor the 50ms deadline");
}
