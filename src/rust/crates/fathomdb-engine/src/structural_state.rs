use rusqlite::{Connection, OptionalExtension};

use crate::dependency_trace;
use crate::search_types::StructuralDependencyStateV1;

pub(crate) fn structural_dependency_state(
    tx: &Connection,
    write_cursor: u64,
    effective_at: i64,
) -> rusqlite::Result<StructuralDependencyStateV1> {
    structural_dependency_state_inner(tx, write_cursor, effective_at, None, false)
}

pub(crate) fn structural_dependency_state_as_of(
    tx: &Connection,
    write_cursor: u64,
    effective_at: i64,
    as_of_cursor: u64,
    include_out_of_window: bool,
) -> rusqlite::Result<StructuralDependencyStateV1> {
    structural_dependency_state_inner(
        tx,
        write_cursor,
        effective_at,
        Some(as_of_cursor),
        include_out_of_window,
    )
}

fn structural_dependency_state_inner(
    tx: &Connection,
    write_cursor: u64,
    effective_at: i64,
    as_of_cursor: Option<u64>,
    include_out_of_window: bool,
) -> rusqlite::Result<StructuralDependencyStateV1> {
    let owner: Option<(String, String)> = tx
        .query_row(
            "SELECT artifact_role, completeness FROM _fathomdb_artifact_revisions \
             WHERE write_cursor=?1 AND schema_version=1",
            [i64::try_from(write_cursor).map_err(|_| rusqlite::Error::InvalidQuery)?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((role, completeness)) = owner else {
        return Ok(StructuralDependencyStateV1::NotApplicable);
    };
    if role != "derived_semantic" || completeness != "complete" {
        return Ok(StructuralDependencyStateV1::NotApplicable);
    }
    let registered = match as_of_cursor {
        None => dependency_trace::registered_dependency_for_cursor(tx, write_cursor, effective_at),
        Some(as_of) => dependency_trace::registered_dependency_for_cursor_as_of(
            tx,
            write_cursor,
            effective_at,
            as_of,
            include_out_of_window,
        ),
    }
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(if registered {
        StructuralDependencyStateV1::Registered
    } else {
        StructuralDependencyStateV1::NotRegistered
    })
}
