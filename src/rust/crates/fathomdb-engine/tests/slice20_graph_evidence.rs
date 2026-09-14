//! Slice 20 contract tests for exact frozen graph evidence.

use fathomdb_engine::{
    GraphEvidenceArtifactV1, GraphEvidenceRefV1, GraphEvidenceResolveRequestV1,
    GraphEvidenceSidecarEntryV1, GraphEvidenceSidecarV1, ResolvedGraphEvidenceV1,
};

#[test]
fn public_graph_evidence_carriers_are_available() {
    let _reference = GraphEvidenceRefV1::new("opaque").unwrap();
    let _entry: Option<GraphEvidenceSidecarEntryV1> = None;
    let _sidecar: Option<GraphEvidenceSidecarV1> = None;
    let _request: Option<GraphEvidenceResolveRequestV1> = None;
    let _artifact: Option<GraphEvidenceArtifactV1> = None;
    let _resolved: Option<ResolvedGraphEvidenceV1> = None;
}
