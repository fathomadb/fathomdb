use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};

/// Deterministic normalized768 recipe retained from exp_ac013_page_cache.
pub struct VaryingEmbedder;

impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("varying", "perf-gates-dense", 768)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in text.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        if h == 0 {
            h = 0xdeadbeef_cafebabe;
        }
        let mut state = h;
        let mut vector = Vec::with_capacity(768);
        for _ in 0..768 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            vector.push(((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0));
        }
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut vector {
            *x /= norm;
        }
        Ok(vector)
    }
}
