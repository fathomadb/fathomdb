#[cfg(feature = "default-embedder")]
pub mod loader;

pub enum EmbedderEvent {
    Loaded,
}
