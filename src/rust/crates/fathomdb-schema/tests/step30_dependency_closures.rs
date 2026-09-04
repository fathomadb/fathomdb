use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::{params, Connection};

fn user_version(connection: &Connection) -> u32 {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

fn insert_closure(connection: &Connection, id_digit: char, phase: &str) -> rusqlite::Result<usize> {
    let id = format!("_fdb:c:{}", id_digit.to_string().repeat(64));
    let fingerprint = id_digit.to_string().repeat(64);
    let physical = matches!(phase, "at_rest_pending" | "complete");
    connection.execute(
        "INSERT INTO _fathomdb_dependency_closures(\
           schema_version,closure_operation_id,root_kind,root_value,cause,\
           effective_at_epoch_s,admitted_write_boundary,\
           admitted_dependency_generation,closure_sequence,retry_fingerprint,\
           phase,affected_count,blocker_code,structural_proof_write_boundary,proof_json\
         ) VALUES(1,?1,'source_revision','source-r1','purged',0,1,1,1,?2,?3,1,\
                  NULL,?4,?5)",
        params![
            id,
            fingerprint,
            phase,
            physical.then_some(1_i64),
            physical.then_some(
                r#"{"schema_version":1,"proof_write_boundary":1,"current_active_dependent_nodes":0,"current_derived_edges":0,"view_eligible_dependents":0,"ownerless_projection_rows":0,"post_admission_registrations":0}"#
            ),
        ],
    )
}

#[test]
fn step30_adds_content_free_closure_state_and_checked_sequence() {
    assert_eq!(SCHEMA_VERSION, 30);
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..29]).unwrap();
    migrate(&connection).unwrap();

    assert_eq!(user_version(&connection), 30);
    let sequence: String = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state \
             WHERE key='_fathomdb_closure_sequence'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sequence, "0");
    assert!(insert_closure(&connection, 'a', "proving").is_err());
    assert!(insert_closure(&connection, 'a', "at_rest_pending").is_ok());
}

#[test]
fn retry_fingerprint_is_unique_only_while_nonterminal() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    insert_closure(&connection, 'a', "complete").unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='1' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();

    let second_id = format!("_fdb:c:{}", "b".repeat(64));
    let duplicate = connection.execute(
        "INSERT INTO _fathomdb_dependency_closures(\
           schema_version,closure_operation_id,root_kind,root_value,cause,\
           effective_at_epoch_s,admitted_write_boundary,\
           admitted_dependency_generation,closure_sequence,retry_fingerprint,\
           phase,affected_count,blocker_code,structural_proof_write_boundary,proof_json\
         ) SELECT 1,?1,root_kind,root_value,cause,1,2,1,2,retry_fingerprint,\
                  'at_rest_pending',1,NULL,2,replace(proof_json,'\"proof_write_boundary\":1','\"proof_write_boundary\":2')\
           FROM _fathomdb_dependency_closures WHERE closure_sequence=1",
        [second_id],
    );
    assert!(duplicate.is_ok(), "a completed operation must release retry-key uniqueness");

    let third_id = format!("_fdb:c:{}", "c".repeat(64));
    let conflicting = connection.execute(
        "INSERT INTO _fathomdb_dependency_closures(\
           schema_version,closure_operation_id,root_kind,root_value,cause,\
           effective_at_epoch_s,admitted_write_boundary,\
           admitted_dependency_generation,closure_sequence,retry_fingerprint,\
           phase,affected_count,blocker_code,structural_proof_write_boundary,proof_json\
         ) SELECT 1,?1,root_kind,root_value,cause,2,3,1,3,retry_fingerprint,\
                  'at_rest_pending',1,NULL,3,replace(proof_json,'\"proof_write_boundary\":2','\"proof_write_boundary\":3')\
           FROM _fathomdb_dependency_closures WHERE closure_sequence=2",
        [third_id],
    );
    assert!(conflicting.is_err(), "only one nonterminal retry owner is allowed");
}
