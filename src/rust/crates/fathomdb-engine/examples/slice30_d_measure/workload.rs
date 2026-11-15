//! Fixed corpus/query streams preserving the canonical perf_gates generator semantics.
use super::{CORPUS_SEED, NATURAL_CALLS, QUERY_COUNT, QUERY_SEED};

pub(crate) fn corpus_bodies(count: usize) -> impl Iterator<Item = String> {
    let vocab = perf_vocab();
    let cumulative = zipf_cumulative(vocab.len());
    let mut rng = SeededRng::new(CORPUS_SEED);
    (0..count).map(move |_| synth_chunk_body(&mut rng, &vocab, &cumulative))
}

pub(crate) fn query_bodies() -> Vec<String> {
    let vocab = perf_vocab();
    let cumulative = zipf_cumulative(vocab.len());
    let mut rng = SeededRng::new(QUERY_SEED);
    (0..QUERY_COUNT).map(|_| synth_chunk_body(&mut rng, &vocab, &cumulative)).collect()
}

pub(crate) fn natural_queries(queries: &[String]) -> impl Iterator<Item = &str> {
    queries.iter().cycle().take(NATURAL_CALLS).map(String::as_str)
}

struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_in(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

/// Vocabulary of synthetic English-like tokens. Size ~1024 keeps the
/// FTS index dense; tokens are short, ASCII, lowercase, deterministic.
fn perf_vocab() -> Vec<String> {
    let mut out = Vec::with_capacity(1024);
    for i in 0..1024 {
        let a = (b'a' + ((i / 26 / 26) % 26) as u8) as char;
        let b = (b'a' + ((i / 26) % 26) as u8) as char;
        let c = (b'a' + (i % 26) as u8) as char;
        out.push(format!("{a}{b}{c}{i:04}"));
    }
    out
}

/// Sample a token index under a Zipfian-ish distribution with shape s=1.0,
/// using inverse-CDF on a precomputed cumulative weight table. Returns an
/// index into the vocabulary in [0, vocab_size).
fn zipf_index(rng: &mut SeededRng, cumulative: &[f64]) -> usize {
    let r = rng.next_f64() * cumulative[cumulative.len() - 1];
    match cumulative.binary_search_by(|w| w.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => idx,
        Err(idx) => idx.min(cumulative.len() - 1),
    }
}

fn zipf_cumulative(vocab_size: usize) -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(vocab_size);
    let mut acc = 0.0_f64;
    for k in 1..=vocab_size {
        acc += 1.0_f64 / k as f64;
        cumulative.push(acc);
    }
    cumulative
}

/// Generate one synthetic chunk body of approximately `target_bytes`
/// (~500 B per ADR) using the Zipfian sampler. Tokens are space-joined.
fn synth_chunk_body(rng: &mut SeededRng, vocab: &[String], cumulative: &[f64]) -> String {
    // Tokens average ~7 chars + 1 separator, target ~500 B => ~63 tokens.
    let mut body = String::with_capacity(512);
    let token_count = 55 + rng.next_in(20);
    for i in 0..token_count {
        if i > 0 {
            body.push(' ');
        }
        body.push_str(&vocab[zipf_index(rng, cumulative)]);
    }
    body
}
