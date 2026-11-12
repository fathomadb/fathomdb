//! Tool-owned verification of a fresh closed template's physical indexed coverage.

use fathomdb_embedder_api::EmbedderIdentity;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
struct KindCoverage {
    cursors: Vec<i64>,
    mapping_rows: usize,
    physical_vectors: usize,
    span_rows: usize,
    known_ends: usize,
    unknown_ends: usize,
}

/// Verify physical coverage against the declared profile without assuming cursor=rowid.
/// Returns per-kind and per-row observations. Panics on incomplete/extra coverage,
/// invalid dimensions/spans/profile, or unreadable closed SQLite state.
pub(super) fn coverage(store: &Path, expected: &EmbedderIdentity) -> Value {
    let connection = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let profile: (String, String, u32) = connection.query_row(
        "SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).unwrap();
    assert_eq!(
        (&profile.0, &profile.1, profile.2),
        (&expected.name, &expected.revision, expected.dimension)
    );
    let canonical: Vec<(String, String, i64, String)> = connection.prepare(
        "SELECT logical_id,kind,write_cursor,body FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL ORDER BY write_cursor"
    ).unwrap().query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
        .unwrap().collect::<Result<_,_>>().unwrap();
    assert!(!canonical.is_empty(), "template must contain active canonical rows");
    let mut kinds = BTreeMap::<String, KindCoverage>::new();
    let mut canonical_keys = BTreeSet::new();
    let mut physical_ids = BTreeSet::new();
    let mut observations = Vec::new();
    for (id, kind, cursor, body) in &canonical {
        assert!(canonical_keys.insert((kind, cursor)), "duplicate canonical cursor within kind");
        let kind_profile: String = connection
            .query_row("SELECT profile FROM _fathomdb_vector_kinds WHERE kind=?1", [kind], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kind_profile, "default");
        let mapping: Vec<i64> = connection
            .prepare("SELECT rowid FROM _fathomdb_vector_rows WHERE kind=?1 AND write_cursor=?2")
            .unwrap()
            .query_map(rusqlite::params![kind, cursor], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(mapping.len(), 1, "missing or duplicate canonical-to-vector mapping");
        let rowid = mapping[0];
        assert!(physical_ids.insert(rowid), "physical vector shared by distinct canonical cursors");
        let physical: Vec<(String, Vec<u8>)> = connection
            .prepare("SELECT kind,embedding FROM vector_default WHERE rowid=?1")
            .unwrap()
            .query_map([rowid], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(physical.len(), 1, "missing or duplicate physical vector");
        assert_eq!(&physical[0].0, kind);
        assert_eq!(physical[0].1.len(), usize::try_from(expected.dimension).unwrap() * 4);
        let spans: Vec<(i64,i64,Option<i64>,usize)> = connection.prepare(
            "SELECT window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans WHERE vector_rowid=?1 AND write_cursor=?2"
        ).unwrap().query_map(rusqlite::params![rowid,cursor],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
            .unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(spans.len(), 1, "missing or duplicate mapped vector span");
        let (window, start, end, length) = spans[0];
        assert_eq!((window, start, length), (0, 0, body.len()));
        if let Some(end) = end {
            let end = usize::try_from(end).expect("nonnegative span end");
            assert!(end <= body.len() && body.is_char_boundary(end), "invalid UTF8 head span end");
        }
        let counts = kinds.entry(kind.clone()).or_default();
        counts.cursors.push(*cursor);
        counts.mapping_rows += mapping.len();
        counts.physical_vectors += physical.len();
        counts.span_rows += spans.len();
        if end.is_some() {
            counts.known_ends += 1;
        } else {
            counts.unknown_ends += 1;
        }
        observations.push(json!({"logical_id":id,"kind":kind,"write_cursor":cursor,"vector_rowid":rowid,
            "dimension":profile.2,"window_number":window,"byte_start":start,"byte_end":end,"body_length":length}));
    }
    // Complete per-cursor joins plus exact global counts reject unmatched extras.
    for table in ["_fathomdb_vector_rows", "vector_default", "_fathomdb_vector_spans"] {
        let count: usize = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, canonical.len(), "unmatched extra mapping/vector/span rows in {table}");
    }
    let by_kind: Vec<_> = kinds.into_iter().map(|(kind,counts)| {
        json!({"kind":kind,"canonical_rows":counts.cursors.len(),"cursors":counts.cursors,
            "mapping_rows":counts.mapping_rows,"physical_vectors":counts.physical_vectors,
            "span_rows":counts.span_rows,"known_ends":counts.known_ends,"unknown_ends":counts.unknown_ends})
    }).collect();
    json!({"verified":true,"by_kind":by_kind,"rows":observations,
        "profile":{"name":profile.0,"revision":profile.1,"dimension":profile.2}})
}
