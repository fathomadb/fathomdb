use super::*;

const TABLES: [&str; 9] = [
    "_fathomdb_artifact_revisions",
    "_fathomdb_source_versions",
    "_fathomdb_source_links",
    "_fathomdb_source_dependencies",
    "_fathomdb_dependency_closures",
    "_fathomdb_actuation_receipts",
    "_fathomdb_actuation_receipt_source_refs",
    "operational_mutations",
    "operational_state",
];

fn lengths(c: &Connection, table: &str) -> Result<String, EngineError> {
    let mut s =
        c.prepare(&format!("PRAGMA table_info({table})")).map_err(|_| EngineError::Storage)?;
    let columns = s
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    if columns.is_empty() {
        return Err(EngineError::Storage);
    }
    Ok(columns
        .iter()
        .map(|name| format!("COALESCE(length(CAST(\"{}\" AS BLOB)),0)", name.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join("+"))
}

pub(crate) fn metadata_bytes(
    c: &Connection,
    table: &str,
    predicate: &str,
    args: &[&dyn rusqlite::ToSql],
) -> Result<u64, EngineError> {
    c.query_row(
        &format!("SELECT COALESCE(SUM({}),0) FROM {table} WHERE {predicate}", lengths(c, table)?),
        args,
        |r| r.get(0),
    )
    .map_err(|_| EngineError::Storage)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Totals {
    rows: u64,
    bytes: u64,
    largest: u64,
    vectors: u64,
    metadata: [u64; 9],
}

impl Totals {
    fn read(c: &Connection) -> Result<Self, EngineError> {
        let _timer = physical_metrics::stage("envelope_full_reconstruction");
        let (rows,bytes,largest) = c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(body AS BLOB))),0),COALESCE(MAX(length(CAST(body AS BLOB))),0) FROM (SELECT body FROM canonical_nodes UNION ALL SELECT body FROM canonical_edges)", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_| EngineError::Storage)?;
        let vectors = c
            .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        let mut metadata = [0; 9];
        for (i, table) in TABLES.iter().enumerate() {
            metadata[i] = metadata_bytes(c, table, "1", &[])?;
        }
        Ok(Self { rows, bytes, largest, vectors, metadata })
    }

    fn inserted(&mut self, c: &Connection, class: &str, cursor: u64) -> Result<(), EngineError> {
        let _timer = physical_metrics::stage("envelope_changed_rows");
        let table = if class == "node" { "canonical_nodes" } else { "canonical_edges" };
        let bytes: u64=c.query_row(&format!("SELECT COALESCE(length(CAST(body AS BLOB)),0) FROM {table} WHERE write_cursor=?1"), [cursor], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
        self.rows = self.rows.checked_add(1).ok_or(EngineError::Storage)?;
        self.bytes = self.bytes.checked_add(bytes).ok_or(EngineError::Storage)?;
        self.largest = self.largest.max(bytes);
        let added = metadata_bytes(
            c,
            TABLES[0],
            "artifact_class=?1 AND write_cursor=?2",
            &[&class, &cursor],
        )?;
        self.metadata[0] = self.metadata[0].checked_add(added).ok_or(EngineError::Storage)?;
        Ok(())
    }

    fn check(&self, c: &Connection) -> Result<(), EngineError> {
        let pages: u64 =
            c.query_row("PRAGMA page_count", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
        let size: u64 =
            c.query_row("PRAGMA page_size", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
        if pages.saturating_mul(size) > 1 << 30 {
            return Err(admission::refused());
        }
        let metadata = self
            .metadata
            .iter()
            .try_fold(0u64, |n, v| n.checked_add(*v))
            .ok_or(EngineError::Storage)?;
        if self.rows > 100_000
            || self.bytes > 256 << 20
            || self.largest > 2 << 20
            || self.vectors > 1_000_000
            || metadata > 64 << 20
        {
            return Err(admission::refused());
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Token {
    connection: usize,
    changes: u64,
    version: u64,
    schema: u64,
    temp_schema: u64,
}
impl Token {
    fn read(c: &Connection) -> Result<Self, EngineError> {
        let pragma = |sql| c.query_row(sql, [], |r| r.get(0)).map_err(|_| EngineError::Storage);
        Ok(Self {
            connection: c as *const Connection as usize,
            changes: c.total_changes(),
            version: pragma("PRAGMA data_version")?,
            schema: pragma("PRAGMA main.schema_version")?,
            temp_schema: pragma("PRAGMA temp.schema_version")?,
        })
    }
}

pub(crate) struct Certificate {
    token: Token,
    binding: String,
    totals: Totals,
    zero: bool,
}
impl Certificate {
    pub(crate) fn committed(&mut self, c: &Connection) {
        // Virtual-table commit may flush shadow rows. External commits do not
        // affect this connection's count; retain the pre-COMMIT data_version.
        self.token.changes = c.total_changes();
    }
}

fn binding(c: &Connection) -> Result<String, EngineError> {
    c.query_row("SELECT COALESCE((SELECT value FROM _fathomdb_open_state WHERE key=?1),'')||'/'||COALESCE((SELECT value FROM _fathomdb_open_state WHERE key=?2),'')", params![ROOT_KEY,FTS_KEY], |r| r.get(0)).map_err(|_| EngineError::Storage)
}

pub(crate) struct Working {
    totals: Totals,
    zero: bool,
}
impl Working {
    pub(crate) fn begin(
        c: &Connection,
        old: Option<Certificate>,
        modeled: bool,
    ) -> Result<Option<Self>, EngineError> {
        if !modeled {
            return Ok(None);
        }
        let Some(debt) = root(c)? else {
            return Ok(None);
        };
        if debt.version != 2 || debt.redacted {
            return Ok(None);
        }
        let token = Token::read(c)?;
        let binding = binding(c)?;
        if let Some(old) = old {
            if old.token == token && old.binding == binding {
                return Ok(Some(Self { totals: old.totals, zero: old.zero }));
            }
        }
        // Failure of optional preproof must retain the original post-effect error boundary.
        let zero = validate_zero(c, &debt).is_ok();
        Ok(Totals::read(c).ok().map(|totals| Self { totals, zero }))
    }

    pub(crate) fn finish(
        &mut self,
        c: &Connection,
        base: u64,
        last: u64,
        batch: &[PreparedWrite],
        removed: u64,
    ) -> Result<(), EngineError> {
        for (i, write) in batch.iter().enumerate() {
            let class = match write {
                PreparedWrite::Node { .. } => "node",
                PreparedWrite::Edge { .. } => "edge",
                _ => return Err(EngineError::Storage),
            };
            self.totals.inserted(c, class, base.saturating_add(i as u64 + 1))?;
        }
        self.totals.metadata[7] =
            self.totals.metadata[7].checked_sub(removed).ok_or(EngineError::Storage)?;
        self.totals.check(c)?;
        let mut debt = root(c)?.ok_or(EngineError::Storage)?;
        // The whitelist creates no mapped vector IDs and only projects new cursors.
        if base < debt.safe_boundary || debt.cursors.iter().any(|v| *v as u64 > base) {
            self.zero = false;
        }
        debt.safe_boundary = debt.safe_boundary.max(last);
        store(c, ROOT_KEY, &debt)?;
        if !self.zero {
            validate_zero(c, &debt)?;
            self.zero = true;
        }
        Ok(())
    }

    pub(crate) fn seal(self, c: &Connection) -> Result<Certificate, EngineError> {
        Ok(Certificate {
            token: Token::read(c)?,
            binding: binding(c)?,
            totals: self.totals,
            zero: self.zero,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_totals_follow_actual_rows_and_history() {
        let c = vector_storage::slice70_mapping_fixture(false);
        let before = Totals::read(&c).unwrap();
        c.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,source_id,logical_id) VALUES(50,'doc',?1,'other','new')", ["é\0z"]).unwrap();
        c.execute("INSERT INTO _fathomdb_artifact_revisions(schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness) VALUES(1,'new','node',50,'legacy','migrated_incomplete')", []).unwrap();
        let mut delta = before.clone();
        delta.inserted(&c, "node", 50).unwrap();
        assert_eq!(delta, Totals::read(&c).unwrap());
        c.execute("UPDATE canonical_nodes SET superseded_at=51 WHERE write_cursor=50", []).unwrap();
        assert_eq!(delta, Totals::read(&c).unwrap());
    }

    #[test]
    fn drift_rejects_same_connection_rollback_and_schema_changes() {
        let c = vector_storage::slice70_mapping_fixture(false);
        c.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,source_id) VALUES(50,'doc','before','other')", []).unwrap();
        let token = Token::read(&c).unwrap();
        c.execute_batch("BEGIN; UPDATE canonical_nodes SET body='changed'; ROLLBACK;").unwrap();
        assert_ne!(token, Token::read(&c).unwrap());
        let token = Token::read(&c).unwrap();
        c.execute_batch("CREATE TEMP TABLE unknown(x)").unwrap();
        assert_ne!(token, Token::read(&c).unwrap());
    }
    fn pending(c: &Connection) {
        erase_row_projections(c, &[10_i64][..]).unwrap();
        let debt = RootDebt {
            version: 2,
            retry: "a".repeat(64),
            cursors: vec![10],
            vectors: vec![101, 102],
            boundary: 10,
            safe_boundary: 10,
            generation: load_dependency_generation(c).unwrap(),
            closures: vec![],
            denied_sources: vec![admission::deny_hash("source", "erased")],
            denied_identities: vec![],
            redacted: false,
            blocker: None,
        };
        store(c, ROOT_KEY, &debt).unwrap();
        store(
            c,
            FTS_KEY,
            &Sanitation { version: 1, phase: "cleaning".into(), family: 0, started: false },
        )
        .unwrap();
    }

    fn commit(
        c: &mut Connection,
        cache: &mut Option<Certificate>,
        base: u64,
        body: String,
        cap: u64,
    ) -> Result<(u64, bool, Vec<ClosureOperationId>), crate::write_commit::CommitBatchError> {
        let write = PreparedWrite::Node {
            kind: "doc".into(),
            body,
            source_id: SourceId::new("other").unwrap(),
            logical_id: Some("survivor".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        };
        crate::write_commit::commit_batch(
            c,
            &[write],
            &[crate::write_validation::WritePlan::Node],
            base,
            cap,
            &[],
            cache,
            #[cfg(feature = "test-hooks")]
            false,
        )
    }

    #[test]
    fn warm_real_commit_preserves_totals_and_reduces_vm_work() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        let work = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter = work.clone();
        c.progress_handler(
            1,
            Some(move || {
                counter.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
        let metrics = Arc::new(physical_metrics::PhysicalMetrics::default());
        metrics.enable_for_test();
        let _scope = physical_metrics::scope(&metrics);
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "first".into(), 0).unwrap();
        let cold = work.swap(0, Ordering::Relaxed);
        assert_eq!(cache.as_ref().unwrap().token, Token::read(&c).unwrap());
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
        work.store(0, Ordering::Relaxed);
        let scans = metrics.count_for_test("envelope_full_reconstruction");
        let proofs = metrics.count_for_test("zero_proof");
        commit(&mut c, &mut cache, 11, "é\0second".into(), 0).unwrap();
        let warm = work.load(Ordering::Relaxed);
        assert_eq!(metrics.count_for_test("envelope_full_reconstruction"), scans);
        assert_eq!(metrics.count_for_test("zero_proof"), proofs);
        assert_eq!(metrics.count_for_test("envelope_changed_rows"), 2);
        eprintln!("certified commit VM steps cold={cold}, warm={warm}; warm envelope reconstructions=0, full zero proofs=0");
        assert!(warm < cold, "cold={cold},warm={warm}");
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
        validate_zero(&c, &root(&c).unwrap().unwrap()).unwrap();
        c.progress_handler(0, None::<fn() -> bool>).unwrap();
    }

    #[test]
    fn cold_retention_can_bring_metadata_below_cap_and_rollback_invalidates() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        for i in 1..=4 {
            c.execute("INSERT INTO operational_mutations(collection_name,record_key,op_kind,payload_json,schema_id,write_cursor) VALUES('ordinary',?1,'append',?2,'s',?1)",params![i,"x".repeat(17<<20)]).unwrap();
        }
        assert!(envelope(&c).is_err());
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "survivor".into(), 1).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
        envelope(&c).unwrap();
        c.execute_batch("BEGIN; UPDATE canonical_nodes SET body='rollback'; ROLLBACK").unwrap();
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
        let restored = Working::begin(&tx, cache.take(), true).unwrap().unwrap();
        assert_eq!(restored.totals, Totals::read(&tx).unwrap());
    }

    #[test]
    fn external_commit_between_commit_and_publication_is_not_rebaselined() {
        let original = vector_storage::slice70_mapping_fixture(false);
        pending(&original);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("race.db");
        original.execute("VACUUM INTO ?1", [path.to_str().unwrap()]).unwrap();
        let mut c = Connection::open(&path).unwrap();
        c.execute_batch("PRAGMA journal_mode=WAL").unwrap();
        let external = Connection::open(&path).unwrap();
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "survivor".into(), 0).unwrap();
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
        let working = Working::begin(&tx, cache.take(), true).unwrap().unwrap();
        assert!(working.zero);
        let mut sealed = working.seal(&tx).unwrap();
        tx.commit().unwrap();
        external
            .execute(
                "UPDATE canonical_nodes SET body='external body growth' WHERE write_cursor=11",
                [],
            )
            .unwrap();
        sealed.committed(&c);
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
        let rebuilt = Working::begin(&tx, Some(sealed), true).unwrap().unwrap();
        assert_eq!(rebuilt.totals, Totals::read(&tx).unwrap());
        assert_eq!(rebuilt.totals.largest, 20);
    }

    #[test]
    fn opposite_class_rows_are_not_counted_twice() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        c.execute("INSERT INTO canonical_edges(write_cursor,kind,from_id,to_id,source_id) VALUES(11,'doc','a','b','other')",[]).unwrap();
        c.execute("INSERT INTO _fathomdb_artifact_revisions(schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness) VALUES(1,'edge-existing','edge',11,'legacy','migrated_incomplete')",[]).unwrap();
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "node".into(), 0).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
    }

    #[test]
    fn actual_commit_failure_discards_certificate_and_rolls_back_rows() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "first".into(), 0).unwrap();
        let before = Totals::read(&c).unwrap();
        c.commit_hook(Some(|| true)).unwrap();
        assert!(commit(&mut c, &mut cache, 11, "abort".into(), 0).is_err());
        c.commit_hook(None::<fn() -> bool>).unwrap();
        assert!(cache.is_none());
        assert!(c.is_autocommit());
        assert_eq!(before, Totals::read(&c).unwrap());
        commit(&mut c, &mut cache, 11, "retry".into(), 0).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
    }

    #[test]
    fn maximum_deletion_and_same_connection_bypass_reconstruct() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "longest-body".into(), 0).unwrap();
        c.execute("DELETE FROM canonical_nodes WHERE write_cursor=11", []).unwrap();
        commit(&mut c, &mut cache, 11, "x".into(), 0).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
        c.execute("UPDATE operational_state SET payload_json='unexpected'", []).unwrap();
        c.execute(
            "UPDATE canonical_nodes SET body='new maximum through bypass' WHERE write_cursor=12",
            [],
        )
        .unwrap();
        commit(&mut c, &mut cache, 12, "z".into(), 0).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
    }

    #[test]
    fn generated_committed_sequences_match_full_reference() {
        use proptest::prelude::*;
        let mut runner =
            proptest::test_runner::TestRunner::new(proptest::test_runner::Config::with_cases(16));
        runner
            .run(&proptest::collection::vec(".{0,80}", 1..8), |bodies| {
                let mut c = vector_storage::slice70_mapping_fixture(false);
                pending(&c);
                let mut cache = None;
                for (i, body) in bodies.into_iter().enumerate() {
                    commit(&mut c, &mut cache, 10 + i as u64, body, 0).unwrap();
                    prop_assert_eq!(&cache.as_ref().unwrap().totals, &Totals::read(&c).unwrap());
                    envelope(&c).unwrap();
                    validate_zero(&c, &root(&c).unwrap().unwrap()).unwrap();
                }
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn inserted_row_probes_use_actual_indexed_work() {
        let c = vector_storage::slice70_mapping_fixture(false);
        let mut probe=c.prepare("SELECT COALESCE(SUM(length(CAST(revision_id AS BLOB))),0) FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2").unwrap();
        probe.query_row(params!["node", 50], |r| r.get::<_, u64>(0)).unwrap();
        assert_eq!(probe.get_status(rusqlite::StatementStatus::FullscanStep), 0);
        assert!(probe.get_status(rusqlite::StatementStatus::VmStep) > 0);
    }
    #[test]
    fn mixed_same_batch_and_unnamed_null_edges_match_reference() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        c.execute("INSERT INTO _fathomdb_projection_registry(name,roles,fts_tokenizer) VALUES('owner',?1,'')",["filterable,searchable"]).unwrap();
        let node = |body: &str| PreparedWrite::Node {
            kind: "doc".into(),
            body: body.into(),
            source_id: SourceId::new("other").unwrap(),
            logical_id: Some("survivor".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        };
        let edge = PreparedWrite::Edge {
            kind: "link".into(),
            from: "survivor".into(),
            to: "other".into(),
            source_id: SourceId::new("other").unwrap(),
            logical_id: None,
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        };
        let batch = [node(r#"{"owner":"old"}"#), edge, node(r#"{"owner":"é new"}"#)];
        let mut cache = None;
        crate::write_commit::commit_batch(
            &mut c,
            &batch,
            &[
                crate::write_validation::WritePlan::Node,
                crate::write_validation::WritePlan::Edge,
                crate::write_validation::WritePlan::Node,
            ],
            10,
            0,
            &[],
            &mut cache,
            #[cfg(feature = "test-hooks")]
            false,
        )
        .unwrap();
        assert_eq!(cache.as_ref().unwrap().totals, Totals::read(&c).unwrap());
        validate_zero(&c, &root(&c).unwrap().unwrap()).unwrap();
        assert_eq!(cache.as_ref().unwrap().totals.rows, 3);
        let properties: u64=c.query_row("SELECT COUNT(*) FROM property_search_index WHERE attr_name='owner' AND write_cursor=13",[],|r|r.get(0)).unwrap();
        assert_eq!(properties, 1);
        let old: u64 = c
            .query_row(
                "SELECT COUNT(*) FROM property_search_index WHERE write_cursor=11",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old, 0);
    }

    #[test]
    fn each_resource_threshold_is_exact() {
        let c = vector_storage::slice70_mapping_fixture(false);
        let limits = [100_000, 256 << 20, 2 << 20, 1_000_000, 64 << 20];
        for (axis, limit) in limits.iter().enumerate() {
            for delta in [-1i64, 0, 1] {
                let mut totals =
                    Totals { rows: 0, bytes: 0, largest: 0, vectors: 0, metadata: [0; 9] };
                let value = (*limit as i64 + delta) as u64;
                match axis {
                    0 => totals.rows = value,
                    1 => totals.bytes = value,
                    2 => totals.largest = value,
                    3 => totals.vectors = value,
                    _ => totals.metadata[0] = value,
                };
                assert_eq!(totals.check(&c).is_ok(), delta <= 0, "axis={axis} delta={delta}");
            }
        }
        // SQLite's actual body-byte oracle is the boundary, including Unicode/NUL.
        for len in [(2 << 20) - 1, 2 << 20, (2 << 20) + 1] {
            c.execute("DELETE FROM canonical_nodes", []).unwrap();
            c.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,source_id) VALUES(50,'doc',?1,'other')",["x".repeat(len)]).unwrap();
            assert_eq!(Totals::read(&c).unwrap().check(&c).is_ok(), envelope(&c).is_ok());
        }
    }

    #[test]
    fn scope_generation_and_lost_certificate_force_exact_reconstruction() {
        let mut c = vector_storage::slice70_mapping_fixture(false);
        pending(&c);
        let mut cache = None;
        commit(&mut c, &mut cache, 10, "survivor".into(), 0).unwrap();
        let mut debt = root(&c).unwrap().unwrap();
        debt.denied_sources.push(admission::deny_hash("source", "another"));
        store(&c, ROOT_KEY, &debt).unwrap();
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
        let metrics = Arc::new(physical_metrics::PhysicalMetrics::default());
        metrics.enable_for_test();
        let _scope = physical_metrics::scope(&metrics);
        let rebuilt = Working::begin(&tx, cache.take(), true).unwrap().unwrap();
        assert_eq!(rebuilt.totals, Totals::read(&tx).unwrap());
        assert_eq!(metrics.count_for_test("zero_proof"), 1);
        let lost = Working::begin(&tx, None, true).unwrap().unwrap();
        assert_eq!(lost.totals, rebuilt.totals);
        debt.generation += 1;
        store(&tx, ROOT_KEY, &debt).unwrap();
        let changed = Working::begin(&tx, None, true).unwrap().unwrap();
        assert!(!changed.zero);
    }
}
