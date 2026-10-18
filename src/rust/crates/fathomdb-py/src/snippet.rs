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
            max_bytes: checked("max_bytes", self.max_bytes)?,
            max_windows: checked("max_windows", self.max_windows)?,
            head_bytes: checked("head_bytes", self.head_bytes)?,
            highlights: self.highlights,
            max_point_queries: checked("max_point_queries", self.max_point_queries)?,
        })
    }
}

fn checked<T: TryFrom<i64>>(name: &str, value: i64) -> PyResult<T> {
    T::try_from(value).map_err(|_| {
        engine_error_to_py(RustEngineError::InvalidArgument {
            msg: format!("snippet {name} is out of range, got {value}"),
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

impl PySnippetOptionsArg {
    /// The options for `snippet_for`, which never fails: each value
    /// saturates into its field type, and the selector clamps it into range.
    fn to_rust_clamped(&self) -> RustSnippetOptions {
        RustSnippetOptions {
            max_bytes: u32::try_from(self.max_bytes.max(0)).unwrap_or(u32::MAX),
            max_windows: u8::try_from(self.max_windows.max(0)).unwrap_or(u8::MAX),
            head_bytes: u32::try_from(self.head_bytes.max(0)).unwrap_or(u32::MAX),
            highlights: self.highlights,
            max_point_queries: u8::try_from(self.max_point_queries.max(0)).unwrap_or(u8::MAX),
        }
    }
}

/// Spans that are not `u32` byte ranges cannot lie inside a body; they are
/// ignored, as the selector ignores spans that run past the body.
fn representable_spans(spans: &[(i64, i64)]) -> Vec<(u32, u32)> {
    spans
        .iter()
        .filter_map(|&(start, end)| Some((u32::try_from(start).ok()?, u32::try_from(end).ok()?)))
        .collect()
}

/// EP-17 pure selector (`fathomdb_engine::snippet_for`): the snippet of
/// `body` for match `spans` (UTF-8 byte ranges). Never fails; out-of-range
/// options are clamped.
#[pyfunction]
#[pyo3(signature = (body, spans, options))]
pub(super) fn snippet_for(
    body: &str,
    spans: Vec<(i64, i64)>,
    options: PySnippetOptionsArg,
) -> PySnippetV1 {
    PySnippetV1::from_rust(&fathomdb_engine::snippet_for(
        body,
        &representable_spans(&spans),
        &options.to_rust_clamped(),
    ))
}

#[cfg(test)]
mod parity_fixture_tests {
    use super::*;
    use serde_json::{json, Value};

    /// The shared parity fixture the Python and TypeScript tests also read:
    /// the binding conversions plus the engine selector must reproduce each
    /// case's `expected` exactly.
    #[test]
    fn snippet_for_reproduces_the_shared_parity_fixture() {
        let path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../../conformance/snippet-for-parity.json");
        let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let int = |options: &Value, name: &str, default: i64| {
            options.get(name).map_or(default, |v| v.as_i64().unwrap())
        };
        let mut mismatches = Vec::new();
        for case in fixture["cases"].as_array().unwrap() {
            let options = &case["options"];
            let options = PySnippetOptionsArg {
                max_bytes: int(options, "max_bytes", 300),
                max_windows: int(options, "max_windows", 1),
                head_bytes: int(options, "head_bytes", 0),
                highlights: options.get("highlights").is_some_and(|v| v.as_bool().unwrap()),
                max_point_queries: int(options, "max_point_queries", 20),
            };
            let spans: Vec<(i64, i64)> = case["spans"]
                .as_array()
                .unwrap()
                .iter()
                .map(|pair| (pair[0].as_i64().unwrap(), pair[1].as_i64().unwrap()))
                .collect();
            let got = snippet_for(case["body"].as_str().unwrap(), spans, options);
            let actual = json!({
                "text": got.text,
                "anchor": got.anchor,
                "windows": got.windows,
                "spans": got.spans,
                "highlights": got.highlights,
                "cut": got.cut,
            });
            if actual != case["expected"] {
                mismatches.push(format!("{}: {actual}", case["name"]));
            }
        }
        assert!(mismatches.is_empty(), "fixture mismatches:\n{}", mismatches.join("\n"));
    }
}
