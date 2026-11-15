//! Private accepted synthetic streams; no database, provider or runtime controls.

pub(crate) fn corpus_bodies(_count: usize) -> impl Iterator<Item = String> {
    std::iter::empty()
}

pub(crate) fn query_bodies() -> Vec<String> {
    Vec::new()
}

pub(crate) fn natural_queries(queries: &[String]) -> impl Iterator<Item = &str> {
    queries.iter().take(0).map(String::as_str)
}
