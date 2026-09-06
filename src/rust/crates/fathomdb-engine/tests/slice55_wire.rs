//! Slice 55 RED contract for canonical version-1 trace/integrity/explanation wire bytes.

use fathomdb_engine::{
    decode_dependency_trace_result_v1, encode_dependency_trace_result_v1,
    DependencyTraceDirectionV1, DependencyTraceEdgeV1, DependencyTraceNodeV1,
    DependencyTraceResultV1, TraceArtifactClassV1, TraceArtifactRoleV1, TraceNodeLifecycleV1,
    TraceReadBoundaryV1,
};
use proptest::prelude::*;

fn result() -> DependencyTraceResultV1 {
    DependencyTraceResultV1 {
        schema_version: 1,
        root_revision_id: "source-r1".into(),
        direction: DependencyTraceDirectionV1::ToDependents,
        nodes: vec![
            DependencyTraceNodeV1 {
                schema_version: 1,
                artifact_revision_id: "source-r1".into(),
                artifact_class: TraceArtifactClassV1::Node,
                role: TraceArtifactRoleV1::CanonicalSource,
                depth: 0,
                lifecycle: TraceNodeLifecycleV1::Node {
                    schema_version: 1,
                    state: fathomdb_engine::LifecycleState::Active,
                    superseded: false,
                    valid_at_effective: true,
                },
            },
            DependencyTraceNodeV1 {
                schema_version: 1,
                artifact_revision_id: "derived-r1".into(),
                artifact_class: TraceArtifactClassV1::Node,
                role: TraceArtifactRoleV1::Derived,
                depth: 1,
                lifecycle: TraceNodeLifecycleV1::Node {
                    schema_version: 1,
                    state: fathomdb_engine::LifecycleState::Active,
                    superseded: false,
                    valid_at_effective: true,
                },
            },
        ],
        dependency_edges: vec![DependencyTraceEdgeV1 {
            schema_version: 1,
            dependency_id: "dep-1".into(),
            source_revision_id: "source-r1".into(),
            derived_revision_id: "derived-r1".into(),
            registered_dependency_generation: 1,
        }],
        checked_work_units: 2,
        complete: true,
        read_boundary: TraceReadBoundaryV1 {
            schema_version: 1,
            effective_at_epoch_s: 1,
            observed_write_boundary: 1,
            dependency_generation: 1,
            projection_generation_id: "pgen1:00000000000000000000000000000000".into(),
        },
    }
}

fn corrupt(value: &serde_json::Value) -> fathomdb_engine::DependencyTraceErrorV1 {
    decode_dependency_trace_result_v1(&serde_json::to_vec(value).unwrap()).unwrap_err()
}

#[test]
fn slice55_wire_v1_canonical_bytes() {
    let bytes = encode_dependency_trace_result_v1(&result()).unwrap();
    let fixture = include_bytes!("fixtures/slice55/trace-v1.json");
    assert_eq!(bytes, fixture);
    assert_eq!(decode_dependency_trace_result_v1(fixture).unwrap(), result());
}

#[test]
fn slice55_wire_rejects_noncanonical_u64_at_exact_nested_path() {
    let bytes = encode_dependency_trace_result_v1(&result()).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["dependencyEdges"][0]["registeredDependencyGeneration"] = "01".into();
    assert_eq!(corrupt(&value).field_path, "/dependencyEdges/0/registeredDependencyGeneration");
}

#[test]
fn slice55_wire_rejects_incoherent_lifecycle_union_at_exact_path() {
    let bytes = encode_dependency_trace_result_v1(&result()).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["nodes"][1]["lifecycle"]["artifactClass"] = "edge".into();
    assert_eq!(corrupt(&value).field_path, "/nodes/1/lifecycle/artifactClass");
}

#[test]
fn slice55_wire_rejects_u32_overflow_and_count_disagreement() {
    let bytes = encode_dependency_trace_result_v1(&result()).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["nodes"][1]["depth"] = serde_json::json!(4_294_967_296_u64);
    assert_eq!(corrupt(&value).field_path, "/nodes/1/depth");

    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["checkedWorkUnits"] = serde_json::json!(1);
    assert_eq!(corrupt(&value).field_path, "/checkedWorkUnits");
}

#[test]
fn slice55_wire_rejects_role_endpoint_and_uniqueness_invariants() {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/slice55/trace-v1.json")).unwrap();

    let mut value = fixture.clone();
    value["nodes"][0]["role"] = "derived".into();
    assert_eq!(corrupt(&value).field_path, "/nodes/0/role");

    let mut value = fixture.clone();
    value["dependencyEdges"][0]["sourceRevisionId"] = "other-r1".into();
    assert_eq!(corrupt(&value).field_path, "/dependencyEdges/0/sourceRevisionId");

    let mut value = fixture;
    value["nodes"][1]["artifactRevisionId"] = "source-r1".into();
    assert_eq!(corrupt(&value).field_path, "/nodes/1/artifactRevisionId");
}

#[test]
fn slice55_wire_rejects_duplicate_dependency_ids_and_noncanonical_order() {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/slice55/trace-v1.json")).unwrap();
    let mut value = fixture;
    let mut second_node = value["nodes"][1].clone();
    second_node["artifactRevisionId"] = "z-derived-r1".into();
    let mut second_edge = value["dependencyEdges"][0].clone();
    second_edge["dependencyId"] = "dep-2".into();
    second_edge["derivedRevisionId"] = "z-derived-r1".into();
    value["nodes"].as_array_mut().unwrap().push(second_node);
    value["dependencyEdges"].as_array_mut().unwrap().push(second_edge);
    value["checkedWorkUnits"] = 3.into();

    let mut duplicate = value.clone();
    duplicate["dependencyEdges"][1]["dependencyId"] = "dep-1".into();
    assert_eq!(corrupt(&duplicate).field_path, "/dependencyEdges/1/dependencyId");

    value["nodes"].as_array_mut().unwrap().swap(1, 2);
    value["dependencyEdges"].as_array_mut().unwrap().swap(0, 1);
    assert_eq!(corrupt(&value).field_path, "/dependencyEdges/1");
}

#[test]
fn slice55_wire_errors_carry_schema_version() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/slice55/trace-v1.json")).unwrap();
    value["complete"] = false.into();
    assert_eq!(corrupt(&value).schema_version, 1);
}

proptest! {
    #[test]
    fn slice55_trace_codec_round_trip(revision in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}") {
        let mut value = result();
        value.root_revision_id = revision.clone();
        value.nodes[0].artifact_revision_id = revision.clone();
        value.dependency_edges[0].source_revision_id = revision;
        let bytes = encode_dependency_trace_result_v1(&value).unwrap();
        prop_assert_eq!(decode_dependency_trace_result_v1(&bytes).unwrap(), value);
    }
}
