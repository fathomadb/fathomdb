#![no_main]

use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    encode_resolved_graph_evidence_v1, ArtifactRevisionId, CanonicalHash, DenseReadiness,
    EmbedderChoice, Engine, EvidenceArtifactLifecycleV1, Filter, FilterTerm,
    GraphEvidenceArtifactV1, InitialState, LifecycleState, PreparedWrite, ProjectionFts,
    ProjectionRole, ProjectionSpec, ProjectionTarget, ProjectionVector, PropertyComparisonOp,
    PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue, ReadContextV1, ReadView,
    ResolvedGraphEvidenceV1, SearchFilter, SourceId, SourceLocator, SourceRevisionId,
};
use libfuzzer_sys::fuzz_target;
use tempfile::TempDir;

struct PropertyFixture {
    _dir: TempDir,
    engine: Engine,
    writes: u64,
}

fn fixture() -> &'static Mutex<PropertyFixture> {
    static FIXTURE: OnceLock<Mutex<PropertyFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_property_predicates");
        let path = dir.path().join("props_fuzz.sqlite");
        let engine = Engine::open_with_choice(&path, EmbedderChoice::None)
            .expect("open property fixture engine")
            .engine;
        let _ = engine.configure_projections(
            &[
                ProjectionSpec {
                    name: "p_int".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_flt".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_str".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_bool".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        );
        Mutex::new(PropertyFixture { _dir: dir, engine, writes: 0 })
    })
}

fn arb_str(u: &mut Unstructured<'_>, max_len: usize) -> String {
    match u.int_in_range(0..=8).unwrap_or(0) {
        0 => String::new(),
        1 => "a\0b".to_string(),
        2 => "a\x1eb".to_string(),
        3 => "p_int".to_string(),
        4 => "p_flt".to_string(),
        5 => "p_str".to_string(),
        6 => "p_bool".to_string(),
        _ => {
            let s: String = u.arbitrary().unwrap_or_default();
            s.chars().take(max_len).collect()
        }
    }
}

fn arb_i64_edge(u: &mut Unstructured<'_>) -> i64 {
    match u.int_in_range(0..=11).unwrap_or(0) {
        0 => i64::MIN,
        1 => i64::MAX,
        2 => 0,
        3 => -1,
        4 => 1,
        5 => (1_i64 << 53) - 1,
        6 => 1_i64 << 53,
        7 => (1_i64 << 53) + 1,
        8 => -(1_i64 << 53) - 1,
        9 => -(1_i64 << 53),
        10 => i64::MIN + 1,
        _ => u.arbitrary().unwrap_or(0),
    }
}

fn arb_f64_edge(u: &mut Unstructured<'_>) -> f64 {
    match u.int_in_range(0..=14).unwrap_or(0) {
        0 => f64::NAN,
        1 => f64::INFINITY,
        2 => f64::NEG_INFINITY,
        3 => -0.0,
        4 => 0.0,
        5 => f64::MIN_POSITIVE,
        6 => -f64::MIN_POSITIVE,
        7 => f64::MAX,
        8 => f64::MIN,
        9 => 9_223_372_036_854_775_808.0,
        10 => -9_223_372_036_854_775_808.0,
        11 => f64::from_bits(1),
        12 => f64::from_bits(0x8000_0000_0000_0001),
        13 => (1_i64 << 53) as f64,
        _ => f64::from_bits(u.arbitrary().unwrap_or(0)),
    }
}

fn arb_prop_value(u: &mut Unstructured<'_>) -> PropertyValue {
    match u.int_in_range(0..=3).unwrap_or(0) {
        0 => PropertyValue::String(arb_str(u, 64)),
        1 => PropertyValue::Int(arb_i64_edge(u)),
        2 => PropertyValue::Float(arb_f64_edge(u)),
        _ => PropertyValue::Bool(u.arbitrary().unwrap_or(false)),
    }
}

fn arb_op(u: &mut Unstructured<'_>) -> PropertyComparisonOp {
    match u.int_in_range(0..=5).unwrap_or(0) {
        0 => PropertyComparisonOp::Eq,
        1 => PropertyComparisonOp::Ne,
        2 => PropertyComparisonOp::Lt,
        3 => PropertyComparisonOp::Lte,
        4 => PropertyComparisonOp::Gt,
        _ => PropertyComparisonOp::Gte,
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let mut u = Unstructured::new(data);

    // 1. Token parsers on arbitrary strings.
    let token = arb_str(&mut u, 32);
    if let Some(op) = PropertyComparisonOp::parse(&token) {
        assert_eq!(PropertyComparisonOp::parse(op.as_str()), Some(op));
    }
    if let Some(cls) = PropertyEntityClass::parse(&token) {
        assert_eq!(PropertyEntityClass::parse(cls.as_str()), Some(cls));
    }
    if let Some(role) = ProjectionRole::from_str_opt(&token) {
        assert_eq!(ProjectionRole::from_str_opt(role.as_str()), Some(role));
    }
    if let Some(ready) = DenseReadiness::from_str_opt(&token) {
        assert_eq!(DenseReadiness::from_str_opt(ready.as_str()), Some(ready));
    }

    // 2. Build PropertyMap with edge values and test JSON encoding + ReadContextV1.
    let map_len = u.int_in_range(0..=8).unwrap_or(0);
    let mut props = PropertyMap::new();
    for _ in 0..map_len {
        props.insert(arb_str(&mut u, 32), arb_prop_value(&mut u));
    }

    let pred_count = u.int_in_range(0..=8).unwrap_or(0);
    let mut preds = Vec::with_capacity(pred_count);
    for _ in 0..pred_count {
        preds.push(PropertyPredicate {
            key: arb_str(&mut u, 32),
            op: arb_op(&mut u),
            value: arb_prop_value(&mut u),
        });
    }

    let mut sf = SearchFilter::default();
    sf.property_predicates = preds.clone();
    let _ = ReadContextV1::new(ReadView::default(), sf.clone());

    // 3. Exercise ResolvedGraphEvidenceV1 JSON encoding with fuzzed properties.
    if let (Ok(art_rev), Ok(src_rev), Ok(hash)) = (
        ArtifactRevisionId::new("art-rev-1"),
        SourceRevisionId::new("src-rev-1"),
        CanonicalHash::sha256("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
    ) {
        let resolved = ResolvedGraphEvidenceV1 {
            schema_version: 1,
            artifact_revision_id: art_rev,
            artifact: GraphEvidenceArtifactV1::Node {
                logical_id: "n1".to_string(),
                kind: "note".to_string(),
                body: "hello".to_string(),
                properties: Some(props.clone()),
            },
            source_id: "src-1".to_string(),
            source_version_id: "sv-1".to_string(),
            source_revision_id: src_rev,
            locator: SourceLocator::whole_body(),
            canonical_source_body: "hello".to_string(),
            evidence_text: "hello".to_string(),
            canonical_source_hash: hash,
            effective_valid_at: 1_700_000_000,
            artifact_lifecycle: EvidenceArtifactLifecycleV1::Node {
                state: LifecycleState::Active,
                superseded: false,
            },
            source_lifecycle_state: LifecycleState::Active,
            dependency: None,
        };
        let _ = encode_resolved_graph_evidence_v1(&resolved);
    }

    // 4. Exercise live Engine property writes, filter lowering, and ProjectionSpec validation.
    let mode = u.int_in_range(0..=2).unwrap_or(0);
    let mut guard = fixture().lock().unwrap_or_else(|p| p.into_inner());
    match mode {
        0 => {
            // Bounded superseding write on logical_id "fuzz-prop" to keep table tiny.
            if guard.writes < 256 {
                if let Ok(src) = SourceId::new("fuzz-prop-src") {
                    let write = PreparedWrite::Node {
                        kind: "note".to_string(),
                        body: r#"{"p_int":1,"p_str":"ok"}"#.to_string(),
                        source_id: src,
                        logical_id: Some(format!("prop-{}", guard.writes % 8)),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        properties: Some(props),
                    };
                    if guard.engine.write(&[write]).is_ok() {
                        guard.writes += 1;
                    }
                }
            }
            let filter = Filter { terms: preds.into_iter().map(FilterTerm::Property).collect() };
            let _ = guard.engine.read_list_filter("note", &filter, 16, &ReadView::default());
        }
        1 => {
            let _ = guard.engine.search_filtered("ok", Some(sf));
        }
        _ => {
            // Fuzz ProjectionSpec validation (invalid combinations must fail closed).
            let mut roles = BTreeSet::new();
            if u.arbitrary().unwrap_or(false) {
                roles.insert(ProjectionRole::Filterable);
            }
            if u.arbitrary().unwrap_or(false) {
                roles.insert(ProjectionRole::Rankable);
            }
            if u.arbitrary().unwrap_or(false) {
                roles.insert(ProjectionRole::Searchable);
            }
            let fts = if u.arbitrary().unwrap_or(false) {
                Some(ProjectionFts {
                    tokenizer: if u.arbitrary().unwrap_or(false) {
                        Some(arb_str(&mut u, 32))
                    } else {
                        None
                    },
                })
            } else {
                None
            };
            let vector = if u.arbitrary().unwrap_or(false) {
                Some(ProjectionVector {
                    embedder: if u.arbitrary().unwrap_or(false) {
                        Some(arb_str(&mut u, 32))
                    } else {
                        None
                    },
                    dense_readiness: None,
                    kinds: if u.arbitrary().unwrap_or(false) {
                        Some(vec![arb_str(&mut u, 24)])
                    } else {
                        None
                    },
                })
            } else {
                None
            };
            let source = if u.arbitrary().unwrap_or(false) {
                let segs = u.int_in_range(0..=4).unwrap_or(0);
                Some((0..segs).map(|_| arb_str(&mut u, 24)).collect())
            } else {
                None
            };
            let entity_class = if u.arbitrary().unwrap_or(true) {
                PropertyEntityClass::Node
            } else {
                PropertyEntityClass::Edge
            };
            let spec = ProjectionSpec {
                name: arb_str(&mut u, 40),
                roles,
                fts,
                vector,
                source,
                entity_class,
            };
            // Always also pass a drop list targeting a nonexistent/fuzzed target to test validation.
            let drop_target = ProjectionTarget::new(arb_str(&mut u, 24), entity_class);
            // Keep our seeded baseline specs intact by rejecting specs that would modify them unless invalid.
            if !matches!(spec.name.as_str(), "p_int" | "p_flt" | "p_str" | "p_bool")
                && !matches!(drop_target.name.as_str(), "p_int" | "p_flt" | "p_str" | "p_bool")
            {
                let _ = guard.engine.configure_projections(&[spec], &[drop_target]);
            }
        }
    }
});
