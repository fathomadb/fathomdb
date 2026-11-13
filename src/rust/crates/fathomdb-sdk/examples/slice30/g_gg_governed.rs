//! Explicit governed GG fixture inventory; retained source and erased derived are distinct.

use super::{g_gg_stages, g_template, CoreObserver, GAdapter};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, InitialState, LifecycleState, PreparedWrite,
    ProvenancedNodeV1, ReadView, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

fn inventory(
    path: &Path,
    expected: &fathomdb_embedder_api::EmbedderIdentity,
    deleted: bool,
) -> Value {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let profile:(String,String,u32)=connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(
        (&profile.0, &profile.1, profile.2),
        (&expected.name, &expected.revision, expected.dimension)
    );
    let canonical:Vec<(String,String,i64,String,String,Option<i64>)>=connection.prepare("SELECT logical_id,kind,write_cursor,state,body,superseded_at FROM canonical_nodes ORDER BY write_cursor").unwrap().query_map([],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).unwrap().collect::<Result<_,_>>().unwrap();
    let mut rows = Vec::new();
    let mut physical_ids = BTreeSet::new();
    let mut active = 0;
    let mut deleted_rows = 0;
    let mut superseded_history = 0;
    let mut retained_deleted_source_vectors = 0;
    let mut closed_derived_without_vectors = 0;
    let mut seen_source = false;
    let mut seen_derived = false;
    for (id, kind, cursor, state, body, superseded) in &canonical {
        superseded_history += usize::from(superseded.is_some());
        assert!(superseded.is_none());
        let closed_derived = deleted && id == "gg-derived";
        let deleted_source = deleted && id == "gg-source";
        assert_eq!(state, if closed_derived || deleted_source { "deleted" } else { "active" });
        active += usize::from(state == "active");
        deleted_rows += usize::from(state == "deleted");
        let mapping: Vec<i64> = connection
            .prepare("SELECT rowid FROM _fathomdb_vector_rows WHERE kind=?1 AND write_cursor=?2")
            .unwrap()
            .query_map(rusqlite::params![kind, cursor], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(mapping.len(), usize::from(!closed_derived));
        let mut spans = 0;
        for rowid in &mapping {
            assert!(physical_ids.insert(*rowid));
            let physical: (String, usize) = connection
                .query_row(
                    "SELECT kind,length(embedding) FROM vector_default WHERE rowid=?1",
                    [rowid],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                (&physical.0, physical.1),
                (kind, usize::try_from(expected.dimension).unwrap() * 4)
            );
            let actual:Vec<(i64,i64,Option<usize>,usize)>=connection.prepare("SELECT window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans WHERE vector_rowid=?1 AND write_cursor=?2").unwrap().query_map(rusqlite::params![rowid,cursor],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().collect::<Result<_,_>>().unwrap();
            assert_eq!(actual.len(), 1);
            let (window, start, end, length) = actual[0];
            assert_eq!((window, start, length), (0, 0, body.len()));
            let end = end.expect("pinned successful BGE head is known");
            assert!(end <= body.len() && body.is_char_boundary(end));
            spans += actual.len();
        }
        let cursor_spans: usize = connection
            .query_row(
                "SELECT count(*) FROM _fathomdb_vector_spans WHERE write_cursor=?1",
                [cursor],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cursor_spans, spans);
        let fts: usize = connection
            .query_row(
                "SELECT count(*) FROM search_index WHERE kind=?1 AND write_cursor=?2",
                rusqlite::params![kind, cursor],
                |row| row.get(0),
            )
            .unwrap();
        let fts_v2: usize = connection
            .query_row(
                "SELECT count(*) FROM search_index_v2 WHERE kind=?1 AND write_cursor=?2",
                rusqlite::params![kind, cursor],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((fts, fts_v2), (usize::from(!closed_derived), usize::from(!closed_derived)));
        if id == "gg-source" {
            assert!(!seen_source);
            seen_source = true;
            retained_deleted_source_vectors += usize::from(
                state == "deleted" && mapping.len() == 1 && spans == 1 && fts == 1 && fts_v2 == 1,
            );
        }
        if id == "gg-derived" {
            assert!(!seen_derived);
            seen_derived = true;
            closed_derived_without_vectors += usize::from(
                state == "deleted" && mapping.is_empty() && spans == 0 && fts == 0 && fts_v2 == 0,
            );
        }
        rows.push(json!({"id":id,"kind":kind,"write_cursor":cursor,"state":state,"superseded_at":superseded,
            "mapping_rows":mapping.len(),"vector_rowids":mapping,"span_rows":spans,"node_fts_rows":fts,"node_fts_v2_rows":fts_v2}));
    }
    assert!(seen_source && seen_derived, "governed source and derived rows must be observed");
    assert_eq!(superseded_history, 0);
    let physical: usize =
        connection.query_row("SELECT count(*) FROM vector_default", [], |row| row.get(0)).unwrap();
    let mapped: usize = connection
        .query_row("SELECT count(*) FROM _fathomdb_vector_rows", [], |row| row.get(0))
        .unwrap();
    let spans: usize = connection
        .query_row("SELECT count(*) FROM _fathomdb_vector_spans", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        (physical, mapped, spans),
        (physical_ids.len(), physical_ids.len(), physical_ids.len())
    );
    let closure:Vec<(String,usize)>=connection.prepare("SELECT phase,affected_count FROM _fathomdb_dependency_closures ORDER BY closure_sequence").unwrap().query_map([],|row|Ok((row.get(0)?,row.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(closure, if deleted { vec![("complete".into(), 1)] } else { Vec::new() });
    json!({"canonical_rows":canonical.len(),"current_by_state":{"active":active,"deleted":deleted_rows},
        "superseded_history":superseded_history,"mapped_vectors":mapped,"physical_vectors":physical,"span_rows":spans,
        "retained_deleted_source_vectors":retained_deleted_source_vectors,"closed_derived_without_vectors":closed_derived_without_vectors,
        "closure":closure,"profile":{"name":profile.0,"revision":profile.1,"dimension":profile.2},"rows":rows})
}

fn readiness(engine: &fathomdb_engine::Engine) -> Value {
    let report = engine.read_embedding_readiness().unwrap();
    assert_eq!(report.state.as_str(), "ready");
    assert!(report.usable_embedder);
    assert_eq!(report.pending_count, 0);
    assert!(report.affected_kinds.is_empty());
    assert!(report.blocked.is_none());
    json!({"state":report.state.as_str(),"usable_embedder":report.usable_embedder,
        "pending_count":report.pending_count,"affected_kinds":report.affected_kinds,
        "blocked":report.blocked.as_ref().map(|blocked|format!("{blocked:?}"))})
}

fn control(engine: &fathomdb_engine::Engine, stage: &str) -> Value {
    let started = Instant::now();
    let view = ReadView { valid_as_of: Some(100), ..ReadView::default() };
    let result = engine
        .search_text_only_filtered_with_snippets("s30gggoverned", None, &view, 10, None)
        .unwrap();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    ids.sort();
    assert_eq!(
        ids,
        if stage == "valid" { vec!["l:gg-derived", "l:gg-source"] } else { Vec::new() }
    );
    json!({"stage":stage,"valid_as_of":100,"ids":ids,"elapsed_ns":elapsed_ns})
}

pub(super) fn fixture(config: &Value, out: &Path, observer: CoreObserver) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    if !g_template::run(config, out) {
        return;
    }
    let source = out.join("case-0/template.sqlite");
    from_template(config, out, &source, observer);
}

pub(super) fn governed_writes() -> [PreparedWrite; 2] {
    let source_body = "s30gggoverned canonical épreuve";
    let hash: String =
        Sha256::digest(source_body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
    [
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "doc".into(),
            body: source_body.into(),
            source_id: SourceId::new("test:slice30-gg-governed").unwrap(),
            logical_id: Some("gg-source".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("gg-source-r1").unwrap(),
                SourceVersionId::new("v1").unwrap(),
            ),
        }),
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "note".into(),
            body: "derived s30gggoverned proof".into(),
            source_id: SourceId::new("test:slice30-gg-governed").unwrap(),
            logical_id: Some("gg-derived".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("gg-derived-r1").unwrap(),
                SourceVersionId::new("v1").unwrap(),
                SourceRevisionId::new("gg-source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(hash).unwrap(),
            ),
        }),
    ]
}

pub(super) fn from_template(
    config: &Value,
    out: &Path,
    source: &Path,
    observer: CoreObserver,
) -> bool {
    let Ok(prepared) = g_gg_stages::prepare_store(config, out, source, "governed", |path| {
        GAdapter::open("engine_hybrid", path)
    }) else {
        return false;
    };
    let g_gg_stages::PreparedStore { adapter, original, directory, store } = prepared;
    let report = adapter.report();
    assert_eq!(
        report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid.as_deref(),
        config["gpu_uuid"].as_str()
    );
    let GAdapter::Core(opened) = &adapter else { unreachable!() };
    let engine = &opened.engine;
    macro_rules! verified {
        ($read:expr) => {{
            match g_gg_stages::verification(config, out, $read) {
                Ok(value) => value,
                Err(()) => {
                    adapter.close();
                    return false;
                }
            }
        }};
    }
    let writes = governed_writes();
    let Ok(write_ns) = g_gg_stages::mutation(config, out, "write", || {
        let writing = Instant::now();
        engine.write(&writes).unwrap();
        u64::try_from(writing.elapsed().as_nanos()).unwrap()
    }) else {
        adapter.close();
        return false;
    };
    let Ok(register_ns) = g_gg_stages::mutation(config, out, "dependency", || {
        let registering = Instant::now();
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("gg-dep", "gg-source-r1", "gg-derived-r1")
                    .unwrap(),
            )
            .unwrap();
        u64::try_from(registering.elapsed().as_nanos()).unwrap()
    }) else {
        adapter.close();
        return false;
    };
    let draining = Instant::now();
    let valid_drain_budget = match g_gg_stages::drain(engine, config) {
        Ok(report) => report,
        Err(error) => {
            std::fs::write(
                out.join("budget.json"),
                serde_json::to_vec_pretty(&json!({
                    "phase":"drain","disposition":"phase_failed","error":error
                }))
                .unwrap(),
            )
            .unwrap();
            adapter.close();
            return false;
        }
    };
    let valid_drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
    let valid_readiness = verified!(|| readiness(engine));
    let valid = verified!(|| { inventory(&store, &report.default_embedder, false) });
    let mut controls = vec![verified!(|| control(engine, "valid"))];
    if !g_gg_stages::measure_stage(
        engine,
        config,
        &directory,
        ("valid", 100, "s30gggoverned"),
        observer,
    ) {
        adapter.close();
        return false;
    }
    let Ok(transition_ns) = g_gg_stages::mutation(config, out, "transition", || {
        let transitioning = Instant::now();
        engine.transition("gg-source", LifecycleState::Deleted, None).unwrap();
        u64::try_from(transitioning.elapsed().as_nanos()).unwrap()
    }) else {
        adapter.close();
        return false;
    };
    let draining = Instant::now();
    let deleted_drain_budget = match g_gg_stages::drain(engine, config) {
        Ok(report) => report,
        Err(error) => {
            std::fs::write(
                out.join("budget.json"),
                serde_json::to_vec_pretty(&json!({
                    "phase":"drain","disposition":"phase_failed","error":error
                }))
                .unwrap(),
            )
            .unwrap();
            adapter.close();
            return false;
        }
    };
    let deleted_drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
    let deleted_readiness = verified!(|| readiness(engine));
    let pending = deleted_readiness["pending_count"].as_u64().unwrap();
    let deleted = verified!(|| { inventory(&store, &report.default_embedder, true) });
    controls.push(verified!(|| control(engine, "deleted")));
    if !g_gg_stages::measure_stage(
        engine,
        config,
        &directory,
        ("deleted", 100, "s30gggoverned"),
        observer,
    ) {
        adapter.close();
        return false;
    }
    adapter.close();
    let source_hash = verified!(|| { Sha256::digest(std::fs::read(source).unwrap()).to_vec() });
    assert_eq!(source_hash, original);
    let mut proof = json!({"controls":controls,"valid_inventory":valid,"deleted_inventory":deleted,"valid_readiness":valid_readiness,"deleted_readiness":deleted_readiness,"selected_cuda_uuid":report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid,
        "governed_write_calls":1,"write_rows":2,"dependency_registrations":1,"transitions":1,"pending":pending,
        "write_ns":write_ns,"register_ns":register_ns,"valid_drain_ns":valid_drain_ns,"transition_ns":transition_ns,"deleted_drain_ns":deleted_drain_ns,
        "scope":"ordinary provenance/dependency/lifecycle/public-view controls and physical inventory; timed stages recorded separately"});
    if config["work_deadline_ns"].is_u64() {
        proof["valid_drain_budget"] = valid_drain_budget;
        proof["deleted_drain_budget"] = deleted_drain_budget;
    }
    std::fs::write(directory.join("controls.json"), serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
    true
}
