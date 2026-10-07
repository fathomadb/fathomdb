//! Argument checks the Python and TypeScript bindings apply before the core.

use fathomdb_engine::{
    Filter, FilterTerm, PageRequestV1, Predicate, PreparedWrite, ProjectionSpec, ScalarValue,
    SearchFilter,
};

use crate::error::{Error, ErrorKind, Result};

/// The largest ranked-result `limit` the SDKs accept.
const MAX_RANKED_LIMIT: usize = 100;

/// AC-068a: an embedded NUL in a content/control string is a
/// `WriteValidation` error before the core sees it. `source_id` instead follows
/// the Engine's identity grammar and may contain NUL.
pub(crate) fn text(value: &str) -> Result<()> {
    if value.as_bytes().contains(&0) {
        return Err(Error::sdk(ErrorKind::WriteValidation, "embedded NUL byte in string argument"));
    }
    Ok(())
}

pub(crate) fn path(path: &std::path::Path) -> Result<()> {
    if path.as_os_str().as_encoded_bytes().contains(&0) {
        return Err(Error::sdk(ErrorKind::WriteValidation, "embedded NUL byte in path argument"));
    }
    Ok(())
}

pub(crate) fn opt_text(value: Option<&str>) -> Result<()> {
    value.map_or(Ok(()), text)
}

pub(crate) fn texts<S: AsRef<str>>(values: &[S]) -> Result<()> {
    values.iter().try_for_each(|value| text(value.as_ref()))
}

pub(crate) fn ranked_limit(name: &str, limit: usize) -> Result<()> {
    if (1..=MAX_RANKED_LIMIT).contains(&limit) {
        Ok(())
    } else {
        Err(Error::invalid_argument(format!(
            "{name} must be an integer in 1..={MAX_RANKED_LIMIT}, got {limit}"
        )))
    }
}

/// Resolve an optional CE blend weight to its 0.3 default; it must be finite.
pub(crate) fn alpha(alpha: Option<f64>) -> Result<f64> {
    let alpha = alpha.unwrap_or(0.3);
    if alpha.is_finite() {
        Ok(alpha)
    } else {
        Err(Error::invalid_argument(format!("alpha must be finite, got {alpha}")))
    }
}

pub(crate) fn search_filter(filter: &SearchFilter) -> Result<()> {
    opt_text(filter.source_type.as_deref())?;
    opt_text(filter.kind.as_deref())?;
    opt_text(filter.status.as_deref())?;
    filter.attributes.iter().try_for_each(|(name, value)| {
        text(name)?;
        text(value)
    })
}

pub(crate) fn filter(filter: &Filter) -> Result<()> {
    filter.terms.iter().try_for_each(|term| match term {
        FilterTerm::SourceType(value) | FilterTerm::Kind(value) | FilterTerm::Status(value) => {
            text(value)
        }
        FilterTerm::Json(predicate) => self::predicate(predicate),
        FilterTerm::CreatedAfter(_) => Ok(()),
    })
}

pub(crate) fn predicate(predicate: &Predicate) -> Result<()> {
    let (Predicate::JsonPathEq { path, value } | Predicate::JsonPathCompare { path, value, .. }) =
        predicate;
    text(path)?;
    match value {
        ScalarValue::Text(value) => text(value),
        ScalarValue::Integer(_) | ScalarValue::Bool(_) => Ok(()),
    }
}

pub(crate) fn page(page: &PageRequestV1) -> Result<()> {
    page.cursor.as_ref().map_or(Ok(()), |cursor| text(&cursor.0))
}

pub(crate) fn projection_spec(spec: &ProjectionSpec) -> Result<()> {
    text(&spec.name)?;
    if let Some(fts) = &spec.fts {
        opt_text(fts.tokenizer.as_deref())?;
    }
    if let Some(vector) = &spec.vector {
        opt_text(vector.embedder.as_deref())?;
    }
    if let Some(source) = &spec.source {
        texts(source)?;
    }
    Ok(())
}

/// Every content/control string of a write except `source_id`, whose closed
/// identity grammar preserves an embedded NUL (AC-068a).
pub(crate) fn prepared_write(write: &PreparedWrite) -> Result<()> {
    match write {
        PreparedWrite::Node { kind, body, logical_id, reason, .. } => {
            text(kind)?;
            text(body)?;
            opt_text(logical_id.as_deref())?;
            opt_text(reason.as_deref())
        }
        PreparedWrite::ProvenancedNode(node) => {
            text(&node.kind)?;
            text(&node.body)?;
            opt_text(node.logical_id.as_deref())?;
            opt_text(node.reason.as_deref())
        }
        PreparedWrite::Edge { kind, from, to, logical_id, body, extractor_model_id, .. } => {
            texts(&[kind, from, to])?;
            opt_text(logical_id.as_deref())?;
            opt_text(body.as_deref())?;
            opt_text(extractor_model_id.as_deref())
        }
        PreparedWrite::ProvenancedEdge(edge) => {
            texts(&[&edge.kind, &edge.from, &edge.to])?;
            opt_text(edge.logical_id.as_deref())?;
            opt_text(edge.body.as_deref())?;
            opt_text(edge.extractor_model_id.as_deref())
        }
        PreparedWrite::OpStore { collection, record_key, schema_id, body } => {
            texts(&[collection, record_key, body])?;
            opt_text(schema_id.as_deref())
        }
        PreparedWrite::AdminSchema { name, kind, schema_json, retention_json } => {
            texts(&[name, kind, schema_json, retention_json])
        }
        // `PreparedWrite` is `#[non_exhaustive]`; the core validates any
        // variant this crate does not know.
        _ => Ok(()),
    }
}
