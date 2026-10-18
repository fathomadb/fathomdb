use super::*;

use fathomdb_engine::{SnippetOptions as RustSnippetOptions, SnippetV1 as RustSnippetV1};

// ===== EP-17 match-centred snippets ===================================

/// The TypeScript `SnippetOptions`: every field optional, an omitted field
/// takes the Rust default. Integers arrive as `i64` so the conversion, not
/// napi's wrapping `u32` read, decides what an out-of-range value means.
#[napi(object)]
pub struct SnippetOptionsInput {
    pub max_bytes: Option<i64>,
    pub max_windows: Option<i64>,
    pub head_bytes: Option<i64>,
    pub highlights: Option<bool>,
    pub max_point_queries: Option<i64>,
}

impl SnippetOptionsInput {
    /// The engine options for a search. A value that does not fit its field
    /// type is `FDB_INVALID_ARGUMENT`, like a value the engine's range check
    /// rejects.
    pub(crate) fn to_rust(&self) -> Result<RustSnippetOptions> {
        let defaults = RustSnippetOptions::default();
        Ok(RustSnippetOptions {
            max_bytes: checked("max_bytes", self.max_bytes, defaults.max_bytes)?,
            max_windows: checked("max_windows", self.max_windows, defaults.max_windows)?,
            head_bytes: checked("head_bytes", self.head_bytes, defaults.head_bytes)?,
            highlights: self.highlights.unwrap_or(defaults.highlights),
            max_point_queries: checked(
                "max_point_queries",
                self.max_point_queries,
                defaults.max_point_queries,
            )?,
        })
    }
}

fn checked<T: TryFrom<i64>>(name: &str, value: Option<i64>, default: T) -> Result<T> {
    let Some(value) = value else { return Ok(default) };
    T::try_from(value).map_err(|_| {
        engine_error_to_napi(RustEngineError::InvalidArgument {
            msg: format!("snippet {name} is out of range, got {value}"),
        })
    })
}

/// One hit's snippet. Ranges are `[start, end]` UTF-8 byte offsets:
/// `windows` and `spans` into the record body, `highlights` into `text`.
#[napi(object)]
pub struct SnippetV1 {
    pub text: String,
    /// `whole`, `match` or `start`.
    pub anchor: String,
    pub windows: Vec<Vec<u32>>,
    pub spans: Vec<Vec<u32>>,
    pub highlights: Vec<Vec<u32>>,
    pub cut: bool,
}

impl SnippetV1 {
    pub(crate) fn from_rust(s: &RustSnippetV1) -> Self {
        let ranges = |ranges: &[(u32, u32)]| ranges.iter().map(|&(a, b)| vec![a, b]).collect();
        Self {
            text: s.text.clone(),
            anchor: s.anchor.as_str().to_string(),
            windows: ranges(&s.windows),
            spans: ranges(&s.spans),
            highlights: ranges(&s.highlights),
            cut: s.cut,
        }
    }
}

/// The binding form of the engine's snippet sidecar; `None` stays absent.
pub(crate) fn snippets_from_rust(snippets: Option<&Vec<RustSnippetV1>>) -> Option<Vec<SnippetV1>> {
    snippets.map(|all| all.iter().map(SnippetV1::from_rust).collect())
}

impl SnippetOptionsInput {
    /// The options for `snippetFor`, which never fails: each value
    /// saturates into its field type, and the selector clamps it into range.
    fn to_rust_clamped(&self) -> RustSnippetOptions {
        let defaults = RustSnippetOptions::default();
        let u32_of =
            |v: Option<i64>, d: u32| v.map_or(d, |v| u32::try_from(v.max(0)).unwrap_or(u32::MAX));
        let u8_of =
            |v: Option<i64>, d: u8| v.map_or(d, |v| u8::try_from(v.max(0)).unwrap_or(u8::MAX));
        RustSnippetOptions {
            max_bytes: u32_of(self.max_bytes, defaults.max_bytes),
            max_windows: u8_of(self.max_windows, defaults.max_windows),
            head_bytes: u32_of(self.head_bytes, defaults.head_bytes),
            highlights: self.highlights.unwrap_or(defaults.highlights),
            max_point_queries: u8_of(self.max_point_queries, defaults.max_point_queries),
        }
    }
}

/// EP-17 pure selector (`fathomdb_engine::snippet_for`): the snippet of
/// `body` for match `spans` (`[start, end]` UTF-8 byte ranges). Spans that are
/// not pairs of `u32` are ignored, as the selector ignores spans that run past
/// the body. Never fails; out-of-range options are clamped.
#[napi(js_name = "snippetFor")]
pub fn snippet_for(
    body: String,
    spans: Vec<Vec<i64>>,
    options: Option<SnippetOptionsInput>,
) -> SnippetV1 {
    let spans: Vec<(u32, u32)> = spans
        .iter()
        .filter_map(|span| match span.as_slice() {
            &[start, end] => Some((u32::try_from(start).ok()?, u32::try_from(end).ok()?)),
            _ => None,
        })
        .collect();
    let options = options.map_or_else(RustSnippetOptions::default, |o| o.to_rust_clamped());
    SnippetV1::from_rust(&fathomdb_engine::snippet_for(&body, &spans, &options))
}
