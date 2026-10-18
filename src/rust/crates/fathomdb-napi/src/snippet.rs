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
    /// Saturates each value into its field type; the engine's range check
    /// then rejects what is out of range.
    pub(crate) fn to_rust(&self) -> RustSnippetOptions {
        let defaults = RustSnippetOptions::default();
        RustSnippetOptions {
            max_bytes: self.max_bytes.map_or(defaults.max_bytes, saturate_u32),
            max_windows: self.max_windows.map_or(defaults.max_windows, saturate_u8),
            head_bytes: self.head_bytes.map_or(defaults.head_bytes, saturate_u32),
            highlights: self.highlights.unwrap_or(defaults.highlights),
            max_point_queries: self
                .max_point_queries
                .map_or(defaults.max_point_queries, saturate_u8),
        }
    }
}

fn saturate_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

fn saturate_u8(value: i64) -> u8 {
    u8::try_from(value.max(0)).unwrap_or(u8::MAX)
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
