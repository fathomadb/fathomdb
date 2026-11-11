//! Explicit serialized-field census. Adding a field requires a presentation
//! decision as well as a schema update; skipped internal cut_anchors is absent.
mod common;

use serde_json::Value;
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
    add(out,prefix,"class code message remedy candidates[] detail busy.reason busy.in_flight busy.queued busy.abandoned busy.stuck");
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
        add(&mut out,"","store epoch validity_instant state.status degradation[].arm degradation[].reason budget.max_tokens budget.used_tokens budget.cut");
    }
    match tool {
        "list_stores" => {
            add(&mut out, "stores[].", "name open readiness default");
            hints(&mut out);
        }
        "use_store" => add(&mut out, "", "store"),
        "orient" => {
            add(&mut out, "projections[].", "name roles[] fts vector");
            add(&mut out,"projection_status.","runtime_embedder_available runtime_unavailability_reason projections[].name projections[].dense_readiness vector_unsupported_kinds[]");
            add(
                &mut out,
                "embedding_readiness.",
                "state usable_embedder pending_count affected_kinds[]",
            );
            add(&mut out, "", "kinds.status counts.status");
        }
        "search" => {
            add(&mut out,"","mode response_id completeness verdict continuation continuation_exact explanation.status explanation.reason");
            add(&mut out,"results[].","ref kind snippet snippet_cut snippet_anchor highlights[][] source_id score arm ce_score");
            add(&mut out,"explanation.trace.","query_chars k rerank_depth pool_n alpha use_graph_arm recency embedder_id ce_active vector_hits text_hits graph_hits dropped_edge_hits");
            add(
                &mut out,
                "explanation.per_hit[].",
                "ref arm vector_rank text_rank graph_rank fused_score ce_score blended",
            );
            hints(&mut out);
        }
        "fetch" => {
            add(&mut out, "items[].", "ref record.kind record.body record.body_cut");
            error(&mut out, "items[].error.");
            hints(&mut out);
        }
        "expand" => {
            add(&mut out, "targets[].", "ref kind hop_count terminal_edge_kind body body_cut");
            add(&mut out, "", "complete bound");
            hints(&mut out);
        }
        "feedback" => add(&mut out, "", "status reason"),
        _ => panic!("unclassified tool {tool}"),
    }
    out
}

#[tokio::test]
async fn every_serialized_field_has_an_explicit_presentation_contract() {
    let harness = common::Harness::start(common::structured_config()).await;
    for tool in harness.tools().await {
        let schema = serde_json::to_value(tool.output_schema.unwrap()).unwrap();
        let mut actual = BTreeSet::new();
        leaves(&schema, &schema, "", &mut actual);
        assert_eq!(actual,expected(&tool.name),"{} serialized paths need an explicit kept/presentation/omission decision in dev/interfaces/mcp.md",tool.name);
        let omitted = actual
            .iter()
            .filter(|path| {
                path.as_str() == "state.status"
                    || path.as_str() == "verdict"
                    || tool.name == "orient"
                        && matches!(path.as_str(), "kinds.status" | "counts.status")
            })
            .map(String::as_str)
            .collect::<Vec<_>>();
        let wanted = match tool.name.as_ref() {
            "list_stores" | "use_store" => vec![],
            "orient" => vec!["counts.status", "kinds.status", "state.status"],
            "search" => vec!["state.status", "verdict"],
            _ => vec!["state.status"],
        };
        assert_eq!(omitted, wanted, "only approved unavailable placeholders are omitted in text");
        assert!(!actual.iter().any(|path| path.contains("cut_anchors")));
    }
    harness.shutdown().await;
}
