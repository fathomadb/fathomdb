//! Explicit serialized-field census. Adding a field requires a presentation
//! decision as well as a schema update; skipped internal cut_anchors is absent.
mod common;

use serde_json::{json, Value};
use std::collections::BTreeSet;

fn leaves(root: &Value, node: &Value, path: &str, out: &mut BTreeSet<String>) {
    if let Some(reference) = node["$ref"].as_str() {
        leaves(root, root.pointer(reference.strip_prefix('#').unwrap()).unwrap(), path, out);
        return;
    }
    let mut nested = false;
    for key in ["anyOf", "oneOf", "allOf"] {
        if let Some(variants) = node[key].as_array() {
            nested = true;
            for variant in variants {
                leaves(root, variant, path, out);
            }
        }
    }
    if let Some(properties) = node["properties"].as_object() {
        nested = true;
        for (key, value) in properties {
            leaves(
                root,
                value,
                &if path.is_empty() { key.clone() } else { format!("{path}.{key}") },
                out,
            );
        }
    }
    if let Some(items) = node.get("items") {
        nested = true;
        leaves(root, items, &format!("{path}[]"), out);
    }
    if let Some(items) = node["prefixItems"].as_array() {
        nested = true;
        for item in items {
            leaves(root, item, &format!("{path}[]"), out);
        }
    }
    if !nested && node["type"] != "null" {
        out.insert(path.to_owned());
    }
}

fn add(out: &mut BTreeSet<String>, prefix: &str, fields: &str) {
    out.extend(fields.split_ascii_whitespace().map(|field| format!("{prefix}{field}")));
}

fn error(out: &mut BTreeSet<String>, prefix: &str) {
    add(
        out,
        prefix,
        "class code message remedy candidates[] detail argument accepted[] \
         refs_not_in_response[] refs_omitted observed_write_boundary \
         busy.reason busy.in_flight busy.queued busy.abandoned busy.stuck",
    );
}

fn hints(out: &mut BTreeSet<String>) {
    add(
        out,
        "hints[].",
        "action reason refs[] refs_omitted mode arm cause continuation max_tokens store",
    );
}

fn expected(tool: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    error(&mut out, "error.");
    if !matches!(tool, "list_stores" | "use_store") {
        add(
            &mut out,
            "",
            "store epoch validity_instant \
             state.status state.database_id state.generation state.nonce \
             state.canonical_write_boundary state.projection_cursor state.drift \
             degradation[].arm degradation[].reason \
             budget.max_tokens budget.used_tokens budget.cut",
        );
    }
    match tool {
        "list_stores" => {
            add(&mut out, "", "default_store");
            add(&mut out, "stores[].", "name open readiness default");
            hints(&mut out);
        }
        "use_store" => add(&mut out, "", "store"),
        "orient" => {
            add(&mut out, "projections[].", "entity_class name roles[] fts vector");
            add(
                &mut out,
                "projection_status.",
                "runtime_embedder_available runtime_unavailability_reason \
                 projections[].entity_class projections[].name projections[].dense_readiness \
                 vector_unsupported_kinds[]",
            );
            add(
                &mut out,
                "embedding_readiness.",
                "state usable_embedder pending_count affected_kinds[]",
            );
            add(&mut out, "", "kinds.status counts.status not_in_this_release[]");
            add(&mut out, "vector_coverage.", "status population reason profile.key profile.name profile.revision profile.dimension records known_records unknown_records truncated_known_records known_body_bytes unrepresented_known_bytes unattributed_heads truncated_known_share.numerator truncated_known_share.denominator unrepresented_known_byte_share.numerator unrepresented_known_byte_share.denominator");
        }
        "search" => {
            add(
                &mut out,
                "",
                "mode response_id completeness verdict continuation continuation_exact \
                 frozen_context explanation.status explanation.reason",
            );
            add(
                &mut out,
                "results[].",
                "ref kind snippet snippet_cut snippet_anchor scored_bytes highlights[][] \
                 source_id score arm ce_score evidence_ref artifact_revision_id",
            );
            add(&mut out, "explanation.trace.", "query_chars k rerank_depth pool_n alpha use_graph_arm recency embedder_id ce_active vector_hits text_hits graph_hits dropped_edge_hits");
            add(
                &mut out,
                "explanation.per_hit[].",
                "ref arm vector_rank text_rank graph_rank fused_score ce_score blended",
            );
            hints(&mut out);
        }
        "fetch" => {
            add(
                &mut out,
                "items[].",
                "ref record.kind record.body record.body_cut \
                 record.artifact_revision_id record.source_id record.source_version_id \
                 record.source_revision_id record.evidence_text record.canonical_source_hash \
                 record.artifact_class record.source_lifecycle_state",
            );
            error(&mut out, "items[].error.");
            hints(&mut out);
        }
        "expand" => {
            add(
                &mut out,
                "targets[].",
                "ref kind hop_count terminal_edge_kind body body_cut \
                 artifact_revision_id graph_evidence_ref \
                 terminal_edge_artifact_revision_id terminal_edge_evidence_ref",
            );
            add(&mut out, "", "complete bound frozen_context");
            hints(&mut out);
        }
        "feedback" => add(&mut out, "", "status reason"),
        "actuate" | "put_latest_state" => {
            add(
                &mut out,
                "",
                "operation_id request_sha256 outcome affected_revision_ids[] \
                 resulting_write_boundary resulting_dependency_generation \
                 pending_projection_write_cursors[]",
            );
        }
        _ => panic!("unclassified tool {tool}"),
    }
    out
}

#[tokio::test]
async fn every_serialized_field_has_an_explicit_presentation_contract() {
    // Check read-only 7-tool surface
    let harness = common::Harness::start(common::structured_config()).await;
    let tools = harness.tools().await;
    assert_eq!(tools.len(), 7);
    for tool in tools {
        let schema = serde_json::to_value(tool.output_schema.unwrap()).unwrap();
        let mut actual = BTreeSet::new();
        leaves(&schema, &schema, "", &mut actual);
        assert_eq!(
            actual,
            expected(&tool.name),
            "{} serialized paths need an explicit kept/presentation/omission decision in dev/interfaces/mcp.md",
            tool.name
        );
        let omitted = actual
            .iter()
            .filter(|path| {
                path.as_str() == "verdict"
                    || tool.name == "orient"
                        && matches!(path.as_str(), "kinds.status" | "counts.status")
            })
            .map(String::as_str)
            .collect::<Vec<_>>();
        let wanted = match tool.name.as_ref() {
            "orient" => vec!["counts.status", "kinds.status"],
            "search" => vec!["verdict"],
            _ => vec![],
        };
        assert_eq!(omitted, wanted, "only approved unavailable placeholders are omitted in text");
        assert!(!actual.iter().any(|path| path.contains("cut_anchors")));
    }
    harness.shutdown().await;

    // Also check write-enabled 9-tool surface (`actuate` + `put_latest_state`)
    let fixture = common::Fixture::new();
    let rw = common::Harness::start(fixture.config(json!({
        "stores": {"w": {"path": "w.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;
    let rw_tools = rw.tools().await;
    assert_eq!(rw_tools.len(), 9);
    for tool in rw_tools {
        let schema = serde_json::to_value(tool.output_schema.unwrap()).unwrap();
        let mut actual = BTreeSet::new();
        leaves(&schema, &schema, "", &mut actual);
        assert_eq!(actual, expected(&tool.name), "write-enabled {}", tool.name);
    }
    rw.shutdown().await;
}
