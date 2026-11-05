#![cfg(feature = "default-embedder")]

use fathomdb_embedder::CandleBgeEmbedder;
use fathomdb_embedder_api::Embedder;

#[test]
fn candle_reports_full_and_truncated_utf8_extents() {
    let provider = CandleBgeEmbedder::new().unwrap();
    for text in ["", "hello", "世界 café  "] {
        assert_eq!(provider.truncation_offset(text).unwrap(), Some(text.len()));
    }
    let text = "hello ".repeat(600);
    let end = provider.truncation_offset(&text).unwrap().expect("known BGE coverage");
    assert_eq!(end, 509 * 6 + 5, "510 content tokens fit between CLS and SEP");
    assert!(text.is_char_boundary(end));
    assert!(end < text.len());
}
