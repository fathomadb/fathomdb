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
    /// The engine options for a search. A value that does not fit its field
    /// type is `InvalidArgumentError`, like a value the engine's range check
    /// rejects.
    pub(super) fn to_rust(&self) -> PyResult<RustSnippetOptions> {
        Ok(RustSnippetOptions {
            max_bytes: checked("max_bytes", self.max_bytes, u32::MAX.into())?,
            max_windows: checked("max_windows", self.max_windows, u8::MAX.into())?,
            head_bytes: checked("head_bytes", self.head_bytes, u32::MAX.into())?,
            highlights: self.highlights,
            max_point_queries: checked(
                "max_point_queries",
                self.max_point_queries,
                u8::MAX.into(),
            )?,
        })
    }
}

fn checked<T: TryFrom<i64>>(name: &str, value: i64, max: u64) -> PyResult<T> {
    T::try_from(value).map_err(|_| {
        engine_error_to_py(RustEngineError::InvalidArgument {
            msg: format!("snippet {name} must be in 0..={max}, got {value}"),
        })
    })
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
