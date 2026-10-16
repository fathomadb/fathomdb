//! Slice 41 (0.8.28) AC41-19 — the role-tagged round body is embedding-neutral.
//!
//! A tagged body puts U+001E plus a role code (U+0010..U+0014) at the start of
//! every part. The pinned tokenizer's `BertNormalizer { clean_text: true }`
//! deletes those control characters before tokenizing, so a tagged body and
//! its plain twin must embed to bit-identical vectors, including a body past
//! the 512-token window (truncation happens after the markers are gone).
//! Markers are written literally here so this crate needs no engine helper.

#![cfg(feature = "default-embedder")]

use fathomdb_embedder::CandleBgeEmbedder;
use fathomdb_embedder_api::Embedder;

const USER: &str = "\u{1e}\u{11}";
const ASSISTANT: &str = "\u{1e}\u{12}";
const TOOL: &str = "\u{1e}\u{14}";

fn twins() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let rounds: [(&str, &str); 17] = [
        ("I adopted a cat named Tom.", "Tom is a lovely name for a cat."),
        ("How many doctors did I see?", "You mentioned a dermatologist and an ENT."),
        ("Vitamin D3 dosage?", "Typical doses range from 600 to 2000 IU."),
        ("", "Assistant-only turn after an empty user turn."),
        ("Empty assistant follows.", ""),
        ("Ünïcödé text and 日本語", "Emoji 😀 and flags 🇫🇷"),
        ("cat", "dog"),
        ("my sister's piano", "pianos need tuning"),
        ("trip to paris", "paris in may"),
        ("budget", "coffee or tea"),
        ("Line one\nline two", "Answer\n\nwith blank lines"),
        ("C", "D3"),
        ("Where did I travel last spring?", "You went to Oslo."),
        ("What is my favourite colour?", "You said blue."),
        ("tabs\tand  spaces", "  leading spaces"),
        ("zebra", "zebra"),
        ("quick question", "quick answer"),
    ];
    for (user, assistant) in rounds {
        out.push((format!("{user}\n{assistant}"), format!("{USER}{user}\n{ASSISTANT}{assistant}")));
    }
    out.push(("a\ntool output".to_string(), format!("{USER}a\n{TOOL}tool output")));
    let long_user = "the quick brown fox jumps over the lazy dog ".repeat(80);
    let long_assistant = "and the dog sleeps in the sun all afternoon ".repeat(40);
    out.push((
        format!("{long_user}\n{long_assistant}"),
        format!("{USER}{long_user}\n{ASSISTANT}{long_assistant}"),
    ));
    out.push(("x".to_string(), format!("{USER}x")));
    out
}

#[test]
fn tagged_and_plain_bodies_embed_bit_identically() {
    if std::env::var("FATHOMDB_SKIP_NETWORK_TESTS").is_ok() {
        eprintln!("SKIP: FATHOMDB_SKIP_NETWORK_TESTS set");
        return;
    }
    let embedder = match CandleBgeEmbedder::new() {
        Ok(embedder) => embedder,
        Err(error) => {
            eprintln!("SKIP: default embedder weights unavailable: {error}");
            return;
        }
    };
    let cases = twins();
    assert_eq!(cases.len(), 20);
    assert!(
        cases.last().is_some() && cases[cases.len() - 2].0.split_whitespace().count() > 512,
        "one body must exceed the 512-token window"
    );
    for (plain, tagged) in &cases {
        let a = embedder.embed(plain).expect("embed plain");
        let b = embedder.embed(tagged).expect("embed tagged");
        assert_eq!(a.len(), b.len());
        let differing = a.iter().zip(&b).filter(|(x, y)| x.to_bits() != y.to_bits()).count();
        assert_eq!(differing, 0, "{differing} components differ for body {plain:.40?}");
    }
}
