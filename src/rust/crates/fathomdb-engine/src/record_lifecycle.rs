use super::*;

/// OPP-12 record-lifecycle Phase-1 (0.8.19 Slice 5) — the existence axis.
///
/// One mutually-exclusive typed enum stored as TEXT in the `canonical_nodes.state`
/// column (schema migration step-20). Semantics (design §2 / plan §1):
///   `Pending` = present + versioned but NOT admitted to default retrieval
///               (quarantine / promotion gate);
///   `Active`  = admitted to default retrieval (the shipped-corpus default);
///   `Deleted` = soft-deleted, retained + recoverable, excluded from default
///               reads, stays indexed behind the flag;
///   `Purged`  = terminal, physically erased.
/// `Deleted`/`Purged` are reachable only through the Phase-2/Slice-10
/// `transition`/`purge` verbs — they can NEVER be a create-time state (see
/// [`InitialState`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Pending,
    Active,
    Deleted,
    Purged,
}

impl LifecycleState {
    /// On-disk `canonical_nodes.state` spelling. Must match the migration step-20
    /// `DEFAULT 'active'` and the `state = 'active'` default-read exclusion.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleState::Pending => "pending",
            LifecycleState::Active => "active",
            LifecycleState::Deleted => "deleted",
            LifecycleState::Purged => "purged",
        }
    }

    /// Parse the on-disk spelling back into the typed enum. `None` for any value
    /// outside the closed vocabulary (a corrupt/foreign `state`).
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(LifecycleState::Pending),
            "active" => Some(LifecycleState::Active),
            "deleted" => Some(LifecycleState::Deleted),
            "purged" => Some(LifecycleState::Purged),
            _ => None,
        }
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10) — the target states legally reachable from
    /// `self` via the `transition` VERB (design §2 legal-transition table). This
    /// is the verb-specific enumeration reported by `IllegalTransitionError.legal`:
    ///   `Pending` → `[Active, Deleted]`   (promote / reject)
    ///   `Active`  → `[Deleted]`           (soft-delete)
    ///   `Deleted` → `[Active]`            (undelete)
    ///   `Purged`  → `[]`                  (terminal; nothing is reachable)
    /// `Purged` is DELIBERATELY excluded even from `Deleted`: reaching `purged` is
    /// the `purge` verb's job (see [`Engine::purge`]), NOT a legal `transition`
    /// target, so reporting it here would mislead a caller into thinking
    /// `transition(deleted → purged)` is legal when it is not. Likewise `Pending`
    /// is create-time-only and is never a `transition` target. Derived directly
    /// from [`is_legal_transition_move`] so this can never drift from the table.
    #[must_use]
    pub fn legal_next_states(self) -> Vec<LifecycleState> {
        [
            LifecycleState::Pending,
            LifecycleState::Active,
            LifecycleState::Deleted,
            LifecycleState::Purged,
        ]
        .into_iter()
        .filter(|&to| is_legal_transition_move(self, to))
        .collect()
    }
}

/// OPP-12 Phase-1 (0.8.19 Slice 5) — the CREATE-TIME subset of [`LifecycleState`].
///
/// A write can only bring a node into existence as `Pending` or `Active` (design
/// §2 / gap-6). You CANNOT create a `Deleted`/`Purged` node — those states are
/// reachable only via the `transition`/`purge` verbs (Slice 10). Making the
/// create-time surface a separate two-variant type is the TYPED rejection: a
/// `deleted`/`purged` create is simply unrepresentable in the Rust API (the SDK
/// bindings map an out-of-subset string to a typed write-validation error).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum InitialState {
    Pending,
    /// The back-compat default: every pre-lifecycle write lands `Active`, matching
    /// the migration step-20 `DEFAULT 'active'`.
    #[default]
    Active,
}

impl InitialState {
    /// On-disk `canonical_nodes.state` spelling for a create-time state.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            InitialState::Pending => "pending",
            InitialState::Active => "active",
        }
    }

    /// The full [`LifecycleState`] this create-time state corresponds to.
    #[must_use]
    pub fn to_lifecycle_state(self) -> LifecycleState {
        match self {
            InitialState::Pending => LifecycleState::Pending,
            InitialState::Active => LifecycleState::Active,
        }
    }

    /// Parse a caller-supplied create-time `state` string into the create-time
    /// subset. `Some(state)` for `"pending"`/`"active"`; `None` for `"deleted"`,
    /// `"purged"`, or any unknown value — the SDK bindings turn `None` into a
    /// typed write-validation rejection (you cannot CREATE a deleted/purged node).
    #[must_use]
    pub fn from_create_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(InitialState::Pending),
            "active" => Some(InitialState::Active),
            _ => None,
        }
    }
}

impl Engine {
    /// OPP-12 Phase-1 (0.8.19 Slice 10) — resolve a lifecycle-verb id argument to
    /// the BARE `logical_id` it addresses, enforcing `Logical`(`l:`)-only
    /// addressability (design §3). An untagged string is taken as a bare
    /// `logical_id` (the `l:` form); an explicit `l:`-prefixed string is stripped
    /// to its value; a `Content`(`h:`) or `Passage`(`p:`) id is a typed
    /// [`EngineError::NotLifecycleAddressable`] refusal (never a panic / no-op).
    pub(crate) fn resolve_lifecycle_target(id: &str) -> Result<String, EngineError> {
        match IdSpace::parse(id) {
            Some(parsed) => match parsed.space {
                IdSpaceKind::Logical => Ok(parsed.value),
                other => Err(EngineError::NotLifecycleAddressable { id_space: other }),
            },
            // Untagged — no id-space prefix; treat as a bare logical_id (l: space).
            None => Ok(id.to_string()),
        }
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, R-TR-1/2) — move a governed node between
    /// existence states per the engine-enforced legal-transition table (design
    /// §2): promote `pending→active`, reject `pending→deleted`, soft-delete
    /// `active→deleted`, undelete `deleted→active`. `to_state` is a full
    /// [`LifecycleState`], but `Pending` (create-time only) and `Purged`
    /// (`purge`-only) are never legal `transition` targets, nor are self-loops or
    /// any move from a non-existent/`purged` row — each returns a typed
    /// [`EngineError::IllegalTransition`] enumerating the legal targets.
    ///
    /// `reason` semantics (design §3 gap-6): promote/undelete CLEAR `reason` to
    /// `NULL` (the row is admitted; no standing cause); reject/soft-delete SET
    /// `reason` to the supplied value (`NULL` allowed but the delete-family
    /// expects it). `reason` is advisory — the engine never interprets it.
    ///
    /// Keys on the BARE `logical_id` (`l:` space only); a `Content`(`h:`) or
    /// `Passage`(`p:`) id raises [`EngineError::NotLifecycleAddressable`].
    /// The state flip mutates the single active (`superseded_at IS NULL`) row.
    /// Search visibility is removed on exclusion and restored on admission.
    /// Attribute projections, which cannot apply a read-side lifecycle filter,
    /// are maintained at rest. FTS/vector shadows remain in place and their read
    /// paths enforce lifecycle eligibility; dependency closure separately purges
    /// closed derived artifacts so they cannot consume bounded candidate slots.
    ///
    /// 0.8.20 Slice 15d fix-2 [P2] — the row-owned ATTRIBUTE projection
    /// (`canonical_attributes` / `property_search_index`) is the exception: it has
    /// NO read-side lifecycle filter (the property-FTS5 table cannot carry one), so
    /// it is maintained AT REST to track the backfill's set
    /// (projected ⟺ active ∧ non-superseded). Promote/undelete PROJECT the declared
    /// attributes; soft-delete PURGES them; reject is a no-op.
    pub fn transition(
        &self,
        logical_id: &str,
        to_state: LifecycleState,
        reason: Option<String>,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let lid = Self::resolve_lifecycle_target(logical_id)?;

        // Settle in-flight projection work first: the async projection worker
        // commits vector/FTS shadows on its OWN connection, so a state flip issued
        // while a worker holds the write lock would SQLITE_BUSY. Draining
        // (unfrozen so any unprojected row completes) leaves the worker idle; a
        // bare state flip enqueues no new projection work.
        //
        // Slice 40 B3 aligns the worker with `commit_batch`, and Slice 30 gives
        // this state flip the same `BEGIN IMMEDIATE` ordering. The drain remains
        // load-bearing because it settles projection work before dependency
        // closure admission and avoids needless contention with worker commits.
        self.drain_for_non_embedding_mutation()?;

        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;

        // The lifecycle state lives on the single active (superseded_at IS NULL)
        // version; a `deleted` row is still that active version, just flagged.
        // fix-2 [P2] — also read its `write_cursor` + `body` so the row-owned
        // attribute projection can be maintained after the state flip.
        let current: Option<(String, i64, String, String, String)> = tx
            .query_row(
                "SELECT state, write_cursor, kind, body, row_kind FROM canonical_nodes \
                 WHERE logical_id = ?1 AND superseded_at IS NULL",
                params![lid],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;

        // A missing active row is an absent/purged node — the terminal `Purged`
        // state for legality purposes (nothing is a legal target from there).
        let from_state = match &current {
            Some((s, _, _, _, _)) => LifecycleState::from_str_opt(s).ok_or(EngineError::Storage)?,
            None => LifecycleState::Purged,
        };

        if !is_legal_transition_move(from_state, to_state) {
            return Err(EngineError::IllegalTransition {
                from_state,
                to_state,
                legal: from_state.legal_next_states(),
            });
        }

        if matches!(to_state, LifecycleState::Active) {
            if let Some((_, _, _, body, _)) = &current {
                validate_nested_projection_sources_for_body(&tx, body)?;
            }
            if let Some((_, cursor, _, _, _)) = &current {
                dependency_closure::guard_derived_reactivation(&tx, *cursor)?;
            }
        }

        if matches!(to_state, LifecycleState::Deleted) {
            if let Some((_, cursor, _, _, _)) = &current {
                if let Some(source_revision) =
                    dependency_closure::source_revision_for_cursor(&tx, *cursor)?
                {
                    dependency_closure::admit_soft_closure(
                        &tx,
                        &source_revision,
                        ClosureCauseV1::SoftDeleted,
                        self.next_cursor.load(Ordering::SeqCst),
                        dependency_closure::SoftClosureMode::Complete,
                    )?;
                }
            }
        }

        // Admit (promote/undelete) → clear reason; exclude (reject/soft-delete) →
        // set the supplied reason. `to_state` is Active or Deleted here.
        let new_reason: Option<String> = match to_state {
            LifecycleState::Active => None,
            _ => reason,
        };
        tx.execute(
            "UPDATE canonical_nodes SET state = ?1, reason = ?2 \
             WHERE logical_id = ?3 AND superseded_at IS NULL",
            params![to_state.as_str(), new_reason, lid],
        )
        .map_err(|_| EngineError::Storage)?;

        if let Some((_, cursor, _, body, _)) = &current {
            purge_row_projections_for_cursor_in(
                &tx,
                *cursor,
                &[ProjectionClass::Attribute, ProjectionClass::PropertyFts],
            )
            .map_err(|_| EngineError::Storage)?;
            if matches!(to_state, LifecycleState::Active) {
                project_node_attributes(&tx, *cursor, body).map_err(|_| EngineError::Storage)?;
                refresh_vector_attr_values_for_row(&tx, *cursor, body)
                    .map_err(|_| EngineError::Storage)?;
            }
        }
        let reopened = if matches!(to_state, LifecycleState::Active) {
            match &current {
                Some((_, cursor, kind, body, row_kind)) => {
                    restore_registered_derived_projections(&tx, *cursor, kind, body, row_kind)?
                }
                None => false,
            }
        } else {
            false
        };
        tx.commit().map_err(|_| EngineError::Storage)?;
        if reopened {
            self.projection_runtime.notify_new_work();
        }
        self.counters.record_admin();
        Ok(())
    }
}
