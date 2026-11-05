use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};

struct LegacyProvider;
impl Embedder for LegacyProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("legacy", "1", 1)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0])
    }
}

#[test]
fn providers_without_coverage_report_unknown() {
    for text in ["", "hello", "世界"] {
        assert_eq!(LegacyProvider.truncation_offset(text).unwrap(), None);
    }
}
