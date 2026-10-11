use fathomdb_engine::{
    EmbedderRequired, EmbeddingOperation, EmbeddingReadiness, EmbeddingReadinessState, Engine,
};

#[test]
fn embedding_readiness_and_event_drain_are_always_available() {
    let _: fn(&Engine) -> _ = Engine::drain_embedder_events;
    let _ = std::any::type_name::<EmbedderRequired>();
    let _ = std::any::type_name::<EmbeddingOperation>();
    let _ = std::any::type_name::<EmbeddingReadiness>();
    let _ = std::any::type_name::<EmbeddingReadinessState>();
}
