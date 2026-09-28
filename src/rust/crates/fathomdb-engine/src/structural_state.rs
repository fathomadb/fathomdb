use rusqlite::{Connection, OptionalExtension};

use crate::{dependency_trace, StructuralDependencyStateV1};

pub(crate) fn structural_dependency_state(
    tx: &Connection,
    write_cursor: u64,
    effective_at: i64,
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
    let registered =
        dependency_trace::registered_dependency_for_cursor(tx, write_cursor, effective_at)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(if registered {
        StructuralDependencyStateV1::Registered
    } else {
        StructuralDependencyStateV1::NotRegistered
    })
}
