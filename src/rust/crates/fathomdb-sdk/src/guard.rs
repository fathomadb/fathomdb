//! Argument checks the Python and TypeScript bindings apply before the core.

use fathomdb_engine::{
    Filter, FilterTerm, PageRequestV1, Predicate, PreparedWrite, ProjectionSpec, ProjectionTarget,
    PropertyComparisonOp, PropertyMap, PropertyPredicate, PropertyValue, ScalarValue, SearchFilter,
    MAX_GRAPH_EDGE_PREDICATES,
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

pub(crate) fn non_empty_identifier(field: &str, value: &str) -> Result<()> {
    text(value)?;
    if value.trim().is_empty() {
        return Err(Error::invalid_argument(format!("{field} must not be empty")));
    }
    if value.chars().any(char::is_control) {
        return Err(Error::invalid_argument(format!(
            "{field} must not contain control characters"
        )));
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

pub(crate) fn property_map(map: Option<&PropertyMap>) -> Result<()> {
    let Some(map) = map else {
        return Ok(());
    };
    for (key, value) in map {
        if key.is_empty() {
            return Err(Error::sdk(ErrorKind::WriteValidation, "property key must not be empty"));
        }
        text(key)?;
        if key.contains('\x1e') {
            return Err(Error::sdk(
                ErrorKind::WriteValidation,
                "record separator byte in property key",
            ));
        }
        match value {
            PropertyValue::String(s) => {
                text(s)?;
                if s.contains('\x1e') {
                    return Err(Error::sdk(
                        ErrorKind::WriteValidation,
                        "record separator byte in property string value",
                    ));
                }
            }
            PropertyValue::Float(f) if !f.is_finite() => {
                return Err(Error::sdk(
                    ErrorKind::WriteValidation,
                    "float property value must be finite",
                ));
            }
            PropertyValue::Int(_) | PropertyValue::Float(_) | PropertyValue::Bool(_) => {}
        }
    }
    Ok(())
}

pub(crate) fn property_predicate(pred: &PropertyPredicate) -> Result<()> {
    text(&pred.key)?;
    if let PropertyValue::String(s) = &pred.value {
        text(s)?;
    }
    if pred.key.is_empty() || pred.key.contains('\x1e') {
        return Err(Error::sdk(
            ErrorKind::InvalidFilter,
            "property predicate key must be non-empty and contain no record separator bytes",
        ));
    }
    match &pred.value {
        PropertyValue::String(s) if s.contains('\x1e') => Err(Error::sdk(
            ErrorKind::InvalidFilter,
            "property predicate string value must not contain record separator bytes",
        )),
        PropertyValue::Float(f) if !f.is_finite() => {
            Err(Error::sdk(ErrorKind::InvalidFilter, "property predicate float must be finite"))
        }
        PropertyValue::Bool(_)
            if matches!(
                pred.op,
                PropertyComparisonOp::Lt
                    | PropertyComparisonOp::Lte
                    | PropertyComparisonOp::Gt
                    | PropertyComparisonOp::Gte
            ) =>
        {
            Err(Error::sdk(
                ErrorKind::InvalidFilter,
                "ordering operators are invalid for boolean properties",
            ))
        }
        _ => Ok(()),
    }
}

pub(crate) fn search_filter(filter: &SearchFilter) -> Result<()> {
    opt_text(filter.source_type.as_deref())?;
    opt_text(filter.kind.as_deref())?;
    opt_text(filter.status.as_deref())?;
    filter.attributes.iter().try_for_each(|(name, value)| {
        text(name)?;
        text(value)
    })?;
    filter.property_predicates.iter().try_for_each(property_predicate)
}

pub(crate) fn graph_arm_options(
    graph_seed_k: usize,
    graph_edge_kinds: &[String],
    graph_edge_predicates: &[PropertyPredicate],
) -> Result<()> {
    ranked_limit("graph_seed_k", graph_seed_k)?;
    for kind in graph_edge_kinds {
        text(kind)?;
        if kind.is_empty() || kind.contains('\x1e') {
            return Err(Error::sdk(
                ErrorKind::InvalidFilter,
                "graph_edge_kinds entries must be non-empty and contain no record separator bytes",
            ));
        }
    }
    if graph_edge_predicates.len() > MAX_GRAPH_EDGE_PREDICATES {
        return Err(Error::sdk(
            ErrorKind::InvalidFilter,
            format!(
                "graph_edge_predicates accepts at most {MAX_GRAPH_EDGE_PREDICATES} predicates, got {}",
                graph_edge_predicates.len()
            ),
        ));
    }
    graph_edge_predicates.iter().try_for_each(property_predicate)
}

pub(crate) fn filter(filter: &Filter) -> Result<()> {
    filter.terms.iter().try_for_each(|term| match term {
        FilterTerm::SourceType(value) | FilterTerm::Kind(value) | FilterTerm::Status(value) => {
            text(value)
        }
        FilterTerm::Json(predicate) => self::predicate(predicate),
        FilterTerm::Property(pred) => property_predicate(pred),
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

pub(crate) fn projection_target(target: &ProjectionTarget) -> Result<()> {
    text(&target.name)
}

pub(crate) fn projection_spec(spec: &ProjectionSpec) -> Result<()> {
    text(&spec.name)?;
    if let Some(fts) = &spec.fts {
        opt_text(fts.tokenizer.as_deref())?;
    }
    if let Some(vector) = &spec.vector {
        opt_text(vector.embedder.as_deref())?;
        if let Some(kinds) = &vector.kinds {
            for kind in kinds {
                text(kind)?;
                if kind.trim().is_empty() {
                    return Err(Error::sdk(
                        ErrorKind::WriteValidation,
                        "vector kind must not be empty or whitespace-only",
                    ));
                }
                if kind.contains('\x1e') {
                    return Err(Error::sdk(
                        ErrorKind::WriteValidation,
                        "record separator byte in vector kind",
                    ));
                }
                if kind == "edge_fact" {
                    return Err(Error::sdk(
                        ErrorKind::WriteValidation,
                        "reserved vector kind 'edge_fact' cannot be enrolled",
                    ));
                }
            }
        }
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
        PreparedWrite::Node { kind, body, logical_id, reason, properties, .. } => {
            text(kind)?;
            text(body)?;
            opt_text(logical_id.as_deref())?;
            opt_text(reason.as_deref())?;
            property_map(properties.as_ref())
        }
        PreparedWrite::ProvenancedNode(node) => {
            text(&node.kind)?;
            text(&node.body)?;
            opt_text(node.logical_id.as_deref())?;
            opt_text(node.reason.as_deref())?;
            property_map(node.properties.as_ref())
        }
        PreparedWrite::Edge {
            kind,
            from,
            to,
            logical_id,
            body,
            extractor_model_id,
            properties,
            ..
        } => {
            texts(&[kind, from, to])?;
            opt_text(logical_id.as_deref())?;
            opt_text(body.as_deref())?;
            opt_text(extractor_model_id.as_deref())?;
            property_map(properties.as_ref())
        }
        PreparedWrite::ProvenancedEdge(edge) => {
            texts(&[&edge.kind, &edge.from, &edge.to])?;
            opt_text(edge.logical_id.as_deref())?;
            opt_text(edge.body.as_deref())?;
            opt_text(edge.extractor_model_id.as_deref())?;
            property_map(edge.properties.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;
    use fathomdb_engine::{
        InitialState, ProjectionRole, ProjectionVector, PropertyEntityClass, SourceId,
    };
    use std::collections::BTreeSet;

    fn node_with_props(properties: Option<PropertyMap>) -> PreparedWrite {
        PreparedWrite::Node {
            kind: "note".into(),
            body: "hello".into(),
            source_id: SourceId::new("src-1").unwrap(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties,
        }
    }

    #[test]
    fn guard_prepared_write_validates_properties() {
        let mut ok_props = PropertyMap::new();
        ok_props.insert("score".into(), PropertyValue::Float(1.5));
        ok_props.insert("count".into(), PropertyValue::Int(42));
        ok_props.insert("flag".into(), PropertyValue::Bool(true));
        ok_props.insert("tag".into(), PropertyValue::String("alpha".into()));
        assert!(prepared_write(&node_with_props(Some(ok_props))).is_ok());

        for (bad_k, bad_v) in [
            ("", PropertyValue::Int(1)),
            ("a\0b", PropertyValue::Int(1)),
            ("a\x1eb", PropertyValue::Int(1)),
            ("k", PropertyValue::String("v\0".into())),
            ("k", PropertyValue::String("v\x1e".into())),
            ("k", PropertyValue::Float(f64::NAN)),
            ("k", PropertyValue::Float(f64::INFINITY)),
            ("k", PropertyValue::Float(f64::NEG_INFINITY)),
        ] {
            let mut m = PropertyMap::new();
            m.insert(bad_k.into(), bad_v);
            let err = prepared_write(&node_with_props(Some(m))).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::WriteValidation, "key={bad_k:?}");
        }
    }

    #[test]
    fn guard_projection_spec_validates_vector_kinds() {
        let make_spec = |kinds: Option<Vec<String>>| ProjectionSpec {
            name: "body_vec".into(),
            roles: BTreeSet::from([ProjectionRole::Searchable]),
            fts: None,
            vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds }),
            source: None,
            entity_class: PropertyEntityClass::Node,
        };
        assert!(projection_spec(&make_spec(Some(vec!["custom:memory".into()]))).is_ok());

        for bad in ["", "   ", "bad\0kind", "bad\x1ekind", "edge_fact"] {
            let err = projection_spec(&make_spec(Some(vec![bad.into()]))).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::WriteValidation, "kind={bad:?}");
        }
    }

    #[test]
    fn guard_property_predicates_validate_keys_values_and_bool_ops() {
        let mut filter = SearchFilter::default();
        filter.property_predicates.push(PropertyPredicate {
            key: "flag".into(),
            op: PropertyComparisonOp::Lt,
            value: PropertyValue::Bool(true),
        });
        assert_eq!(search_filter(&filter).unwrap_err().kind(), ErrorKind::InvalidFilter);

        let mut nul_filter = SearchFilter::default();
        nul_filter.property_predicates.push(PropertyPredicate {
            key: "bad\0".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(1),
        });
        assert_eq!(search_filter(&nul_filter).unwrap_err().kind(), ErrorKind::WriteValidation);
    }
}
