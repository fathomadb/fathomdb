use crate::failure::{Failure, Phase};
use fathomdb_engine::{configure_runtime, RuntimeSqliteMode, MEAN_VEC_PIN_THRESHOLD};
use rusqlite::{Connection, OpenFlags};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}
#[derive(Debug)]
pub(crate) struct Admitted {
    pub(crate) path: PathBuf,
    pub(crate) fingerprint: Fingerprint,
}
pub(crate) fn fingerprint(path: &Path) -> Result<Fingerprint, &'static str> {
    let metadata = path.metadata().map_err(|_| "missing")?;
    Ok(Fingerprint { device: metadata.dev(), inode: metadata.ino() })
}
pub(crate) fn initialize() -> Result<(), &'static str> {
    configure_runtime(RuntimeSqliteMode::Diagnostics).map_err(|_| "runtime_configuration")?;
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        // The statically linked extension has SQLite's extension entry ABI.
        let entry: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(entry));
    });
    Ok(())
}
pub(crate) fn readonly(path: &Path, immutable: bool) -> Result<Connection, &'static str> {
    let path = path.to_str().ok_or("invalid_path")?;
    let encoded: String = path
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"/-_.".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    let mut connection = Connection::open_with_flags(
        format!("file:{encoded}?mode=ro{}", if immutable { "&immutable=1" } else { "" }),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "malformed_layout")?;
    connection.busy_timeout(Duration::from_millis(100)).map_err(|_| "storage")?;
    connection.pragma_update(None, "query_only", true).map_err(|_| "storage")?;
    let start = Instant::now();
    let mut ticks = 0_u64;
    connection
        .progress_handler(
            1000,
            Some(move || {
                ticks += 1;
                ticks > 100_000 || start.elapsed() >= Duration::from_secs(30)
            }),
        )
        .map_err(|_| "progress_handler")?;
    Ok(connection)
}
pub(crate) fn admit(path: &Path) -> Result<Admitted, Failure> {
    preflight_inner(path).map_err(|mut error| {
        error.phase = Phase::Preflight;
        error.status = "refused";
        error.effects_possible = false;
        error
    })
}
#[cfg(test)]
pub(crate) fn preflight(path: &Path) -> Result<Admitted, &'static str> {
    admit(path).map_err(|error| error.error)
}
fn preflight_inner(path: &Path) -> Result<Admitted, Failure> {
    initialize()?;
    let metadata = path.metadata().map_err(|_| "missing")?;
    if !metadata.is_file() {
        return Err(Failure::preflight("not_regular"));
    }
    if metadata.len() == 0 {
        return Err(Failure::preflight("empty"));
    }
    let path = path.canonicalize().map_err(|_| "missing")?;
    for suffix in ["-wal", "-journal"] {
        match std::fs::metadata(format!("{}{suffix}", path.display())) {
            Ok(m) if !m.is_file() || m.len() != 0 => {
                return Err(Failure::preflight("initial_sidecar"))
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(Failure::preflight("initial_sidecar"))
            }
            _ => {}
        }
    }
    let lock_path = format!("{}{}", path.display(), fathomdb_schema::LOCK_SUFFIX);
    let lock =
        OpenOptions::new().read(true).write(true).open(lock_path).map_err(|_| "missing_lock")?;
    lock.try_lock().map_err(|_| "locked")?;
    let mut header = [0_u8; 100];
    File::open(&path)
        .and_then(|mut f| f.read_exact(&mut header))
        .map_err(|_| "malformed_header")?;
    if &header[..16] != b"SQLite format 3\0" {
        return Err(Failure::preflight("malformed_header"));
    }
    let observed = u32::from_be_bytes(header[60..64].try_into().unwrap());
    if observed != fathomdb_schema::SCHEMA_VERSION {
        return Err(Failure::schema(observed));
    }
    if u32::from_be_bytes(header[56..60].try_into().unwrap()) != 1 {
        return Err(Failure::preflight("unsupported_encoding"));
    }
    let connection = readonly(&path, true)?;
    validate_layout(&connection)?;
    drop(connection);
    drop(lock);
    Ok(Admitted { fingerprint: fingerprint(&path)?, path })
}
fn manifest_columns(table: &str) -> &'static str {
    match table {
        "canonical_nodes" => "write_cursor,kind,body,source_id,logical_id,superseded_at,row_kind,importance,state,reason,valid_from,valid_until,properties,scope",
        "canonical_edges" => "write_cursor,kind,from_id,to_id,source_id,logical_id,superseded_at,body,t_valid,t_invalid,confidence,extractor_model_id,temporal_fallback,properties,scope",
        "operational_collections" => "name,kind,schema_json,retention_json,format_version,created_at",
        "operational_mutations" => "id,collection_name,record_key,op_kind,payload_json,schema_id,write_cursor,scope",
        "operational_state" => "collection_name,record_key,payload_json,schema_id,write_cursor,scope",
        "_fathomdb_artifact_revisions" => "schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness,scope",
        "_fathomdb_source_links" => "schema_version,artifact_revision_id,source_id,source_version_id,source_revision_id,locator_kind,start_byte,end_byte,hash_algorithm,hash_digest,scope",
        "_fathomdb_source_versions" => "schema_version,source_id,source_version_id,source_revision_id,scope",
        "_fathomdb_source_dependencies" => "schema_version,dependency_id,derived_revision_id,registered_dependency_generation,scope",
        "_fathomdb_dependency_closures" => "schema_version,closure_operation_id,root_kind,root_value,cause,phase,proof_json,closure_sequence,scope",
        "_fathomdb_projection_registry" => "name,roles,fts_tokenizer,vector_embedder,vector_declared,source,entity_class,vector_kinds",
        "canonical_attributes" => "write_cursor,attr_name,attr_value",
        "canonical_property_values" => "entity_class,write_cursor,attr_name,value_type,text_val,num_val,bool_val",
        "_fathomdb_commit_timestamps" => "write_cursor,committed_at_ms",
        "_fathomdb_projection_state" => "kind,last_enqueued_cursor,updated_at",
        "_fathomdb_projection_terminal" => "write_cursor,state",
        "_fathomdb_vector_kinds" => "kind,profile,created_at",
        "_fathomdb_vector_rows" => "rowid,kind,write_cursor",
        "_fathomdb_vector_spans" => "vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length",
        "_fathomdb_erasure_requests" => "ticket,idempotency_key,payload,phase,scope",
        "_fathomdb_embedder_profiles" => "profile,name,revision,dimension,mean_vec",
        "_fathomdb_embed_probe" => "probe_ordinal,probe_text,reference_vec,embedder_name,embedder_revision,dim",
        "_fathomdb_actuation_receipts" => "projection_generation_id,scope",
        "_fathomdb_actuation_receipt_source_refs" => "operation_id,ref_kind,ref_value,scope",
        "_fathomdb_migrations" => "step_id,applied_at_ms",
        "_fathomdb_schema_meta" => "key,value",
        "_fathomdb_read_visibility_state" => "singleton,generation,state_nonce,coalescing",
        "_fathomdb_open_state" => "key,value",
        "_fathomdb_projection_generations" => "schema_version,generation_id,declaration_sha256,transition_boundary,role,origin,retired_boundary",
        "_fathomdb_projection_generation_current" => "singleton,generation_id",
        "search_index" | "search_index_v2" | "search_index_edges" => "body,kind,write_cursor",
        "property_search_index" => "attr_value,attr_name,write_cursor",
        "vector_default" => "embedding,embedding_bin,source_type,kind,created_at,status",
        _ => "1",
    }
}
fn validate_layout(c: &Connection) -> Result<(), &'static str> {
    for entry in fathomdb_schema::TABLE_MANIFEST {
        let table = entry.name;
        let columns = manifest_columns(table);
        c.prepare(&format!("SELECT {columns} FROM {table} LIMIT 0"))
            .map_err(|_| "malformed_layout")?;
    }
    for key in ["search_index_tokenizer_reproject_complete", "tc33_edge_vector_prune_complete"] {
        let value: String = c
            .query_row("SELECT value FROM _fathomdb_open_state WHERE key=?1", [key], |r| r.get(0))
            .map_err(|_| "missing_marker")?;
        if value != "1" {
            return Err("missing_marker");
        }
    }
    let valid: i64 = c.query_row("SELECT count(*) FROM _fathomdb_projection_generation_current c JOIN _fathomdb_projection_generations g USING(generation_id) WHERE c.singleton=1 AND g.role='serving' AND g.schema_version=1 AND length(g.declaration_sha256)=64", [], |r| r.get(0)).map_err(|_| "invalid_generation")?;
    if valid != 1 {
        return Err("invalid_generation");
    }
    let (name, revision, dimension, mean): (String, String, u32, Option<i64>) = c.query_row("SELECT name,revision,dimension,length(mean_vec) FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_| "missing_profile")?;
    if dimension == 0 || mean.is_some_and(|n| n != i64::from(dimension) * 4) {
        return Err("invalid_profile");
    }
    if name != env!("INSPECTOR_DEFAULT_NAME")
        || revision != env!("INSPECTOR_DEFAULT_REVISION")
        || dimension.to_string() != env!("INSPECTOR_DEFAULT_DIMENSION")
    {
        return Err("incompatible_profile");
    }
    let sql: String = c
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='vector_default' AND type='table'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "malformed_vector_shape")?;
    let sql = sql.chars().filter(|ch| !ch.is_whitespace()).collect::<String>().to_ascii_lowercase();
    if !sql.starts_with("createvirtualtablevector_defaultusingvec0(")
        || !sql.contains(&format!("embeddingfloat[{dimension}]"))
        || !sql.contains(&format!("embedding_binbit[{dimension}]"))
        || !sql.contains("source_typetextpartitionkey")
        || !sql.contains("statustext")
        || !sql.contains("scopetext")
    {
        return Err("malformed_vector_shape");
    }
    if name == env!("INSPECTOR_MEAN_NAME") && mean.is_none() {
        let count: u64 = c
            .query_row(
                "SELECT count(*) FROM (SELECT rowid FROM vector_default LIMIT ?1)",
                [MEAN_VEC_PIN_THRESHOLD],
                |r| r.get(0),
            )
            .map_err(|_| "malformed_layout")?;
        if count >= MEAN_VEC_PIN_THRESHOLD {
            return Err("mean_recovery_required");
        }
    }
    let visibility: i64 = c.query_row("SELECT count(*) FROM _fathomdb_read_visibility_state WHERE singleton=1 AND generation>=0 AND length(state_nonce)=64 AND typeof(coalescing)='integer' AND coalescing=0", [], |r| r.get(0)).map_err(|_| "invalid_visibility")?;
    if visibility != 1 {
        return Err("invalid_visibility");
    }
    Ok(())
}
