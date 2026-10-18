use super::*;

use fathomdb_engine::{SnippetOptions as RustSnippetOptions, SnippetV1 as RustSnippetV1};

// ===== EP-17 match-centred snippets ===================================

/// The `fathomdb.SnippetOptions` dataclass as the native layer reads it:
/// one attribute per `SnippetOptions` field. Integers arrive unbounded so
/// the conversion, not PyO3, decides what an out-of-range value means.
#[derive(FromPyObject)]
pub(super) struct PySnippetOptionsArg {
    max_bytes: i64,
    max_windows: i64,
    head_bytes: i64,
    highlights: bool,
    max_point_queries: i64,
}

impl PySnippetOptionsArg {
    /// Saturates each value into its field type; the engine's range check
    /// then rejects what is out of range.
    pub(super) fn to_rust(&self) -> RustSnippetOptions {
        RustSnippetOptions {
            max_bytes: saturate(self.max_bytes),
            max_windows: saturate(self.max_windows),
            head_bytes: saturate(self.head_bytes),
            highlights: self.highlights,
            max_point_queries: saturate(self.max_point_queries),
        }
    }
}

fn saturate<T: TryFrom<i64> + Bounded>(value: i64) -> T {
    T::try_from(value).unwrap_or(if value < 0 { T::MIN } else { T::MAX })
}

trait Bounded {
    const MIN: Self;
    const MAX: Self;
}

impl Bounded for u32 {
    const MIN: Self = u32::MIN;
    const MAX: Self = u32::MAX;
}

impl Bounded for u8 {
    const MIN: Self = u8::MIN;
    const MAX: Self = u8::MAX;
}

/// One hit's snippet. Offsets are UTF-8 byte offsets: `windows` and `spans`
/// into the record body, `highlights` into `text`.
#[pyclass(module = "fathomdb._fathomdb", name = "SnippetV1", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PySnippetV1 {
    text: String,
    anchor: String,
    windows: Vec<(u32, u32)>,
    spans: Vec<(u32, u32)>,
    highlights: Vec<(u32, u32)>,
    cut: bool,
}

impl PySnippetV1 {
    pub(super) fn from_rust(s: &RustSnippetV1) -> Self {
        Self {
            text: s.text.clone(),
            anchor: s.anchor.as_str().to_string(),
            windows: s.windows.clone(),
            spans: s.spans.clone(),
            highlights: s.highlights.clone(),
            cut: s.cut,
        }
    }
}

/// The binding form of the engine's snippet sidecar; `None` stays `None`.
pub(super) fn snippets_from_rust(
    snippets: Option<&Vec<RustSnippetV1>>,
) -> Option<Vec<PySnippetV1>> {
    snippets.map(|all| all.iter().map(PySnippetV1::from_rust).collect())
}
