//! Fixed private synthetic provider shared by the driver and real-store controls.
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};

const DIMENSION: u32 = 768;

pub(crate) struct VaryingEmbedder;
impl VaryingEmbedder {
    fn vector_for(&self, text: &str) -> Vector {
        let dim = DIMENSION as usize;
        // Preserve canonical UTF-8 hashing and f32 operation order.
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in text.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        // Avoid the all-zero xorshift fixed point.
        if h == 0 {
            h = 0xdeadbeef_cafebabe;
        }
        let mut state = h;
        let mut v = Vec::with_capacity(dim);
        for _ in 0..dim {
            // xorshift64 — full-period, zero-pole-avoidance above.
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Preserve the canonical low-32-bit signed cast and f32 rounding.
            let x = ((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0);
            v.push(x);
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut v {
            *x /= norm;
        }
        v
    }
}
impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("varying", "perf-gates-dense", DIMENSION)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        Ok(self.vector_for(text))
    }
}
