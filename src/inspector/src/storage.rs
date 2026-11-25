use crate::policy::Classification;
use crate::session::Session;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

pub(crate) const CHUNK: usize = 256 * 1024;
pub(crate) const RESPONSE: usize = 2 * 1024 * 1024;
const WORK_TIME: std::time::Duration = std::time::Duration::from_secs(25);
struct CatalogBudget {
    bytes: usize,
    returned: usize,
    partial: bool,
}
impl Default for CatalogBudget {
    fn default() -> Self {
        Self { bytes: 65536, returned: 0, partial: false }
    }
}
impl CatalogBudget {
    fn include(&mut self, value: &Value) -> Result<bool, &'static str> {
        let bytes = serde_json::to_vec(value).map_err(|_| "storage")?.len() + 2;
        if self.returned >= 200 || self.bytes + bytes > RESPONSE {
            self.partial = true;
            return Ok(false);
        }
        self.bytes += bytes;
        self.returned += 1;
        Ok(true)
    }
}
#[derive(Default)]
struct AssociationBudget {
    returned: usize,
    bytes: usize,
    examined: usize,
    partial: bool,
}
impl AssociationBudget {
    fn include(&mut self, value: &Value) -> Result<bool, &'static str> {
        let bytes = serde_json::to_vec(value).map_err(|_| "storage")?.len() + 100;
        if self.returned >= 200 || self.bytes + bytes > RESPONSE - 16384 {
            self.partial = true;
            return Ok(false);
        }
        self.returned += 1;
        self.bytes += bytes;
        Ok(true)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Class {
    Node,
    Edge,
    Current,
    Log,
}
impl Class {
    fn table(self) -> &'static str {
        match self {
            Self::Node => "canonical_nodes",
            Self::Edge => "canonical_edges",
            Self::Current => "operational_state",
            Self::Log => "operational_mutations",
        }
    }
    fn body(self) -> &'static str {
        match self {
            Self::Node | Self::Edge => "body",
            _ => "payload_json",
        }
    }
    fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Node => &[
                "kind",
                "logical_id",
                "source_id",
                "scope",
                "row_kind",
                "state",
                "reason",
                "valid_from",
                "valid_until",
                "superseded_at",
                "importance",
            ],
            Self::Edge => &[
                "kind",
                "logical_id",
                "source_id",
                "scope",
                "from_id",
                "to_id",
                "t_valid",
                "t_invalid",
                "superseded_at",
                "confidence",
                "extractor_model_id",
                "temporal_fallback",
            ],
            _ => &["collection_name", "record_key", "schema_id", "scope"],
        }
    }
    fn canonical(self) -> bool {
        matches!(self, Self::Node | Self::Edge)
    }
    fn label(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Edge => "edge",
            Self::Current => "current",
            Self::Log => "log",
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Filter {
    pub(crate) kind: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) collection: Option<String>,
    pub(crate) identity: Option<String>,
    pub(crate) attribute: Option<String>,
}
#[derive(Clone)]
struct Locator {
    class: Class,
    rowid: i64,
    cursor: i64,
    revision_rowid: Option<i64>,
}
#[derive(Clone)]
struct AttributeLocator {
    origin: Locator,
    rowid: i64,
    name: String,
    visibility: (String, String),
    generation: String,
}
pub(crate) struct Reader {
    pub(crate) session: Session,
    pub(crate) attachment: String,
    handles: HashMap<String, Locator>,
    attributes: HashMap<String, AttributeLocator>,
    handle_order: VecDeque<String>,
    sequence: u64,
}
pub(crate) fn nonce() -> String {
    let mut b = [0_u8; 32];
    getrandom::fill(&mut b).expect("OS random source required");
    b.iter().map(|n| format!("{n:02x}")).collect()
}
impl Reader {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            attachment: nonce(),
            handles: HashMap::new(),
            attributes: HashMap::new(),
            handle_order: VecDeque::new(),
            sequence: 0,
        }
    }
    fn mint(&mut self, locator: Locator) -> String {
        let handle = nonce();
        self.handles.insert(handle.clone(), locator);
        self.handle_order.push_back(handle.clone());
        while self.handles.len() + self.attributes.len() > 4000 {
            if let Some(old) = self.handle_order.pop_front() {
                self.handles.remove(&old);
                self.attributes.remove(&old);
            }
        }
        handle
    }
    fn mint_attribute(&mut self, locator: AttributeLocator) -> String {
        let handle = nonce();
        self.attributes.insert(handle.clone(), locator);
        self.handle_order.push_back(handle.clone());
        while self.handles.len() + self.attributes.len() > 4000 {
            if let Some(old) = self.handle_order.pop_front() {
                self.handles.remove(&old);
                self.attributes.remove(&old);
            }
        }
        handle
    }
    fn attribute_chunk(
        &mut self,
        handle: &str,
        field: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Value, &'static str> {
        let locator = self.attributes.get(handle).cloned().ok_or("reference_expired")?;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (origin, classification) = Self::summary(&c, &locator.origin, self.session.policy)?;
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        if context["visibility_generation"].as_str() != Some(locator.visibility.0.as_str())
            || context["visibility_nonce"].as_str() != Some(locator.visibility.1.as_str())
            || context["projection_generation"].as_str() != Some(locator.generation.as_str())
        {
            return Err("reference_expired");
        }
        let column = match field {
            "body" => "attr_value",
            "attribute_name" => "attr_name",
            _ => return Err("unsupported_field"),
        };
        let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM canonical_attributes WHERE rowid=?1 AND write_cursor=?2 AND attr_name=?3)",params![locator.rowid,locator.origin.cursor,locator.name],|r|r.get(0)).map_err(|_|"storage")?;
        if !exists {
            return Err("reference_expired");
        }
        let value = crate::window::Column::open(&c, "canonical_attributes", column, locator.rowid)?;
        let total = value.len();
        let bytes = value.read(offset, limit)?;
        if total.is_some_and(|n| offset > n) {
            return Err("invalid_range");
        }
        let (text, n) = utf8_prefix(bytes.as_deref().unwrap_or_default(), limit)?;
        if n == 0 && total.is_some_and(|total| offset < total) {
            return Err("invalid_utf8_boundary");
        }
        Ok(
            json!({"status":"complete","state":if total.is_none(){"null"}else if total==Some(0){"empty"}else{"value"},"text":text,"offset":offset.to_string(),"total_bytes":total.map(|n|n.to_string()),"continuation":if offset+n<total.unwrap_or(0){Some((offset+n).to_string())}else{None},"context":Self::finished(context),"attribute":{"class":"attribute","attribute_name":locator.name,"origin":origin}}),
        )
    }
    fn context(&mut self, c: &crate::session::ReadConnection) -> Result<Value, &'static str> {
        self.sequence += 1;
        let(generation,state):(String,String)=c.query_row("SELECT CAST(generation AS TEXT),state_nonce FROM _fathomdb_read_visibility_state WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"attachment_expired")?;
        let projection:String=c.query_row("SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",[],|r|r.get(0)).map_err(|_|"attachment_expired")?;
        let dependency:String=c.query_row("SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_dependency_generation'",[],|r|r.get(0)).map_err(|_|"attachment_expired")?;
        Ok(
            json!({"operation":self.sequence.to_string(),"route":"inspector_current_layout","database":self.session.database_id,"attachment":self.attachment,"build":crate::process::BUILD,"protocol":1,"read_started_epoch_ms":c.started_epoch_ms.to_string(),"read_time_epoch_s":c.at_epoch_s.to_string(),"visibility_generation":generation,"visibility_nonce":state,"projection_generation":projection,"dependency_generation":dependency,"write_boundary":{"state":"unknown","reason":"Engine authority not supplied by this route"},"snapshot":"this_request_only","composition":if self.session.cached_cpu{"sealed-cache-cpu-v1"}else{"cleared-env-v2"}}),
        )
    }
    fn finished(mut context: Value) -> Value {
        context["read_ended_epoch_ms"] = json!(crate::session::epoch_ms().to_string());
        context
    }
    fn preview(
        c: &rusqlite::Connection,
        table: &str,
        column: &str,
        rowid: i64,
        characters: usize,
    ) -> Result<Option<String>, &'static str> {
        let value = crate::window::Column::open(c, table, column, rowid)?;
        let bytes = value.read(0, (characters * 4).min(CHUNK))?;
        bytes
            .map(|bytes| {
                utf8_prefix(&bytes, bytes.len())
                    .map(|(text, _)| text.chars().take(characters).collect())
            })
            .transpose()
    }
    fn summary(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        policy: crate::policy::Policy,
    ) -> Result<(Value, Classification), &'static str> {
        let mut remaining = usize::MAX;
        Self::summary_fields_budget(
            c,
            locator,
            locator.class.fields(),
            Some(policy),
            &mut remaining,
        )
    }
    fn inventory_summary(
        c: &crate::session::ReadConnection,
        locator: &Locator,
    ) -> Result<(Value, Classification), &'static str> {
        Self::summary_fields(
            c,
            locator,
            if locator.class.canonical() {
                &["kind", "logical_id", "source_id"]
            } else {
                &["collection_name"]
            },
        )
    }
    fn summary_fields(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        columns: &[&str],
    ) -> Result<(Value, Classification), &'static str> {
        let mut remaining = usize::MAX;
        Self::summary_fields_budget(c, locator, columns, None, &mut remaining)
    }
    fn summary_fields_budget(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        columns: &[&str],
        properties_policy: Option<crate::policy::Policy>,
        remaining: &mut usize,
    ) -> Result<(Value, Classification), &'static str> {
        if *remaining == 0 {
            return Err("budget_exhausted");
        }
        *remaining -= 1;
        let class = locator.class;
        let entries=columns.iter().map(|col|format!("'{col}',CASE WHEN octet_length({col})<=16384 THEN substr(CAST({col} AS TEXT),1,4096) END,'{col}_bytes',octet_length({col}),'{col}_storage',typeof({col})")).collect::<Vec<_>>().join(",");
        let text: Option<String> = c
            .query_row(
                &format!(
                    "SELECT json_object({entries}) FROM {} WHERE rowid=?1 AND write_cursor=?2",
                    class.table()
                ),
                params![locator.rowid, locator.cursor],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "storage")?;
        let mut record: Value =
            serde_json::from_str(&text.ok_or("unavailable")?).map_err(|_| "storage")?;
        for column in columns {
            if record[format!("{column}_storage")] == "real" {
                let value = crate::window::Column::open(c, class.table(), column, locator.rowid)?;
                let bytes = value.read(0, CHUNK)?.ok_or("storage")?;
                record[*column] =
                    json!(String::from_utf8(bytes).map_err(|_| "unsupported_value_encoding")?);
                record[format!("{column}_bytes")] = json!(value.len());
                record[format!("{column}_representation")] =
                    json!("stored_binary64_round_trip; original decimal lexeme unavailable");
            } else if record[format!("{column}_bytes")].as_u64().is_some_and(|n| n > 16384) {
                record[*column] =
                    json!(Self::preview(c, class.table(), column, locator.rowid, 4096)?);
            }
        }
        record["class"] = json!(class);
        record["write_cursor"] = json!(locator.cursor.to_string());
        if class == Class::Log {
            record["entry_id"] = json!(locator.rowid.to_string());
        }
        let mut known = true;
        let mut erased = false;
        let mut inactive = false;
        let mut telemetry = false;
        if class.canonical() {
            let actual:Option<(i64,String,String,String)>=c.query_row("SELECT rowid,substr(revision_id,1,4096),artifact_role,completeness FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),locator.cursor],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|"storage")?;
            if actual.as_ref().map(|r| r.0) != locator.revision_rowid {
                return Err("reference_expired");
            }
            if let Some((_, revision, role, complete)) = actual {
                record["revision_id"] = json!(revision);
                record["artifact_role"] = json!(role);
                record["completeness"] = json!(complete);
            } else {
                record["revision_id"] = Value::Null;
                record["artifact_role"] = json!("unknown");
                record["completeness"] = json!("unknown");
            }
            let lifecycle = if class == Class::Node {
                "superseded_at IS NOT NULL OR state!='active' OR (valid_from IS NOT NULL AND valid_from>?3) OR (valid_until IS NOT NULL AND valid_until<=?3)"
            } else {
                "superseded_at IS NOT NULL OR (t_invalid IS NOT NULL AND t_invalid<=?3)"
            };
            let known_fields = if class == Class::Node {
                "state IN ('active','pending','deleted') AND typeof(valid_from) IN ('null','integer') AND typeof(valid_until) IN ('null','integer') AND typeof(superseded_at) IN ('null','integer')"
            } else {
                "typeof(t_valid) IN ('null','integer') AND typeof(t_invalid) IN ('null','integer') AND typeof(superseded_at) IN ('null','integer')"
            };
            (inactive,known)=c.query_row(&format!("SELECT {lifecycle},COALESCE({known_fields},0) FROM {} WHERE rowid=?1 AND write_cursor=?2",class.table()),params![locator.rowid,locator.cursor,c.at_epoch_s],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"storage")?;
            let closure_sql = "SELECT d.cause FROM _fathomdb_artifact_revisions owner JOIN _fathomdb_source_dependencies dep ON dep.scope=owner.scope AND dep.derived_revision_id=owner.revision_id JOIN _fathomdb_source_links link ON link.scope=owner.scope AND link.artifact_revision_id=owner.revision_id JOIN _fathomdb_source_links origin ON origin.scope=owner.scope AND origin.artifact_revision_id=link.source_revision_id JOIN _fathomdb_dependency_closures d ON d.scope=owner.scope AND d.phase!='complete' AND ((d.root_kind='source_revision' AND d.root_value=link.source_revision_id) OR (d.root_kind='source_bucket' AND d.root_value=origin.source_id)) WHERE owner.artifact_class=?1 AND owner.write_cursor=?2 LIMIT 100001";
            let mut statement = c.prepare(closure_sql).map_err(|_| "storage")?;
            let mut rows =
                statement.query(params![class.label(), locator.cursor]).map_err(|_| "storage")?;
            let mut count = 0;
            while let Some(row) = rows.next().map_err(|_| "storage")? {
                if *remaining == 0 {
                    return Err("budget_exhausted");
                }
                *remaining -= 1;
                count += 1;
                if count > 100000 {
                    known = false;
                    break;
                }
                let cause: String = row.get(0).map_err(|_| "storage")?;
                match cause.as_str() {
                    "purged" | "source_erased" => erased = true,
                    "soft_deleted" | "superseded" => inactive = true,
                    _ => known = false,
                }
            }
            // Queued maintenance intent retains an exact cursor cut without logical removal.
            let queued = c.queued_erasure_contains(locator.cursor)?;
            erased |= queued;
            record["queued_physical_erasure"] = json!(queued);
            record["engine_owned"] =
                json!(record["source_id"].as_str().is_some_and(|s| s.starts_with("_engine:")));
            if class == Class::Node {
                record["structural_role"] = record["row_kind"].take();
                record["structural_role_bytes"] = record["row_kind_bytes"].take();
                record["structural_role_storage"] = record["row_kind_storage"].take();
                record.as_object_mut().ok_or("storage")?.remove("row_kind");
                record.as_object_mut().ok_or("storage")?.remove("row_kind_bytes");
                record.as_object_mut().ok_or("storage")?.remove("row_kind_storage");
            }
        } else {
            telemetry = record["collection_name"] == "erasure_pending_redaction";
            record["mode"] =
                json!(if class == Class::Log { "append_only_log" } else { "latest_state" });
            record["engine_owned"] = json!(matches!(
                record["collection_name"].as_str(),
                Some(
                    "projection_failures"
                        | "erasure_pending_redaction"
                        | "purge_audit"
                        | "excise_source_audit"
                        | "excise_collection_audit"
                )
            ));
        }
        record["inactive_or_history"] = json!(inactive);
        record["retained_erasure_pending"] = json!(erased);
        record["telemetry_payload"] = json!(telemetry);
        let classification = Classification { inactive, erased, telemetry, known };
        if class.canonical() {
            if let Some(policy) = properties_policy {
                if policy.permits(classification) {
                    let (mut preview, bytes, storage): (Option<String>, Option<i64>, String) = c
                        .query_row(
                            &format!(
                                "SELECT CASE WHEN octet_length(properties)<=16384 THEN substr(CAST(properties AS TEXT),1,4096) END,octet_length(properties),typeof(properties) FROM {} WHERE rowid=?1 AND write_cursor=?2",
                                class.table()
                            ),
                            params![locator.rowid, locator.cursor],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                        )
                        .map_err(|_| "storage")?;
                    if bytes.is_some_and(|n| n > 16384) {
                        preview =
                            Self::preview(c, class.table(), "properties", locator.rowid, 4096)?;
                    }
                    record["properties_status"] = json!("available");
                    record["properties"] = json!(preview);
                    record["properties_bytes"] = json!(bytes);
                    record["properties_storage"] = json!(storage);
                } else {
                    record["properties_status"] = json!("refused");
                }
            }
        }
        Ok((record, classification))
    }
    fn display_preview(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        deadline: std::time::Instant,
        budget: &mut usize,
    ) -> Result<Value, &'static str> {
        if std::time::Instant::now() >= deadline {
            return Err("budget_exhausted");
        }
        let unavailable = |state: &str| json!({"state":state,"text":null,"field":null,"complete":false,"source":"bounded_stored_json_field"});
        let inspect = |column_name: &str,
                       budget: &mut usize|
         -> Result<(Option<Value>, &'static str), &'static str> {
            let column =
                crate::window::Column::open(c, locator.class.table(), column_name, locator.rowid)?;
            let Some(total) = column.len() else {
                return Ok((None, "null"));
            };
            if total > 4096 {
                return Ok((None, "oversize_not_inspected"));
            }
            if total > *budget {
                return Ok((None, "preview_budget_exhausted"));
            }
            *budget -= total;
            let bytes = column.read(0, total)?.ok_or("unavailable")?;
            let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
                return Ok((None, "no_display_field"));
            };
            for field in ["title", "name"] {
                if let Some(text) =
                    value.get(field).and_then(Value::as_str).filter(|v| !v.trim().is_empty())
                {
                    let preview = text.chars().take(120).collect::<String>();
                    return Ok((
                        Some(
                            json!({"state":"available","text":preview,"field":field,"complete":text.chars().count()<=120,"source":"bounded_stored_json_field"}),
                        ),
                        "available",
                    ));
                }
            }
            Ok((None, "no_display_field"))
        };
        let (body_preview, body_state) = inspect(locator.class.body(), budget)?;
        if let Some(found) = body_preview {
            return Ok(found);
        }
        if locator.class.canonical() {
            if std::time::Instant::now() >= deadline {
                return Err("budget_exhausted");
            }
            let (props_preview, props_state) = inspect("properties", budget)?;
            if let Some(found) = props_preview {
                return Ok(found);
            }
            if matches!(body_state, "no_display_field" | "null") && props_state != "null" {
                return Ok(unavailable(props_state));
            }
        }
        Ok(unavailable(body_state))
    }
    fn sample_fields(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        deadline: std::time::Instant,
        budget: &mut usize,
    ) -> Result<Value, &'static str> {
        if std::time::Instant::now() >= deadline {
            return Err("budget_exhausted");
        }
        let value = crate::window::Column::open(
            c,
            locator.class.table(),
            locator.class.body(),
            locator.rowid,
        )?;
        let total = value.len();
        let unknown = |state: &str, inspected: bool| json!({"state":state,"inspected":inspected,"names":[],"root_kind":null,"complete_for_this_record":false});
        let Some(total) = total else {
            return Ok(unknown("null", false));
        };
        if total > 65536 {
            return Ok(unknown("oversize_not_inspected", false));
        }
        if total > *budget {
            return Err("budget_exhausted");
        }
        *budget -= total;
        let body = value.read(0, total)?.ok_or("unavailable")?;
        if body.is_empty() {
            return Ok(unknown("empty", true));
        }
        let parsed = crate::tree::children_until(
            |offset, limit| Ok(body[offset..body.len().min(offset + limit)].to_vec()),
            body.len(),
            0,
            None,
            deadline,
            total,
        );
        let tree = match parsed {
            Ok(tree) => tree,
            Err("budget_exhausted") => return Err("budget_exhausted"),
            Err(_) => return Ok(unknown("text_or_unsupported_structure", true)),
        };
        let mut names = vec![];
        let mut complete = tree["status"] == "complete";
        let items = tree["items"].as_array().ok_or("storage")?;
        if items.len() > 20 {
            complete = false;
        }
        if tree["kind"] == "object" {
            for item in items.iter().take(20) {
                let range = &item["key_range"];
                let start = range["start"]
                    .as_str()
                    .ok_or("storage")?
                    .parse::<usize>()
                    .map_err(|_| "storage")?;
                let end = range["end"]
                    .as_str()
                    .ok_or("storage")?
                    .parse::<usize>()
                    .map_err(|_| "storage")?;
                match body
                    .get(start..end)
                    .and_then(|bytes| serde_json::from_slice::<String>(bytes).ok())
                {
                    Some(name) => names.push(name),
                    None => complete = false,
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err("budget_exhausted");
        }
        Ok(
            json!({"state":if complete{"observed"}else{"partial"},"inspected":true,"root_kind":tree["kind"],"names":names,"complete_for_this_record":complete,"field_limit":20,"absence_proof":false}),
        )
    }
    fn attribute_matches(
        c: &rusqlite::Connection,
        cursor: i64,
        name: &str,
        examined: &mut usize,
        attributes: &mut usize,
    ) -> Result<bool, &'static str> {
        let remaining = 100000_usize.saturating_sub(*examined);
        if remaining == 0 {
            return Err("budget_exhausted");
        }
        let mut statement=c.prepare("SELECT rowid,octet_length(attr_name) FROM canonical_attributes INDEXED BY canonical_attributes_cursor_idx WHERE write_cursor=?1 ORDER BY rowid LIMIT ?2").map_err(|_|"storage")?;
        let mut rows = statement.query(params![cursor, remaining as i64]).map_err(|_| "storage")?;
        let mut scanned = 0;
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            *examined += 1;
            *attributes += 1;
            scanned += 1;
            let rowid: i64 = row.get(0).map_err(|_| "storage")?;
            let length: Option<i64> = row.get(1).map_err(|_| "storage")?;
            if length != Some(name.len() as i64) {
                continue;
            }
            let value = crate::window::Column::open(c, "canonical_attributes", "attr_name", rowid)?;
            let mut matches = true;
            for (index, expected) in name.as_bytes().chunks(CHUNK).enumerate() {
                if value.read(index * CHUNK, CHUNK)?.as_deref() != Some(expected) {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Ok(true);
            }
        }
        if scanned == remaining {
            Err("budget_exhausted")
        } else {
            Ok(false)
        }
    }
    pub(crate) fn page(
        &mut self,
        class: Class,
        after: i64,
        limit: usize,
        filter: Filter,
    ) -> Result<Value, &'static str> {
        self.browse(class, after, limit, filter, false)
    }
    pub(crate) fn sample(
        &mut self,
        class: Class,
        after: i64,
        filter: Filter,
    ) -> Result<Value, &'static str> {
        self.browse(class, after, 10, filter, true)
    }
    fn browse(
        &mut self,
        class: Class,
        after: i64,
        limit: usize,
        filter: Filter,
        sample: bool,
    ) -> Result<Value, &'static str> {
        let deadline = std::time::Instant::now() + WORK_TIME;
        if after < 0
            || limit == 0
            || limit > 200
            || (class.canonical() && filter.collection.is_some())
            || (!class.canonical()
                && (filter.kind.is_some() || filter.role.is_some() || filter.source.is_some()))
            || (class != Class::Node && (filter.role.is_some() || filter.attribute.is_some()))
        {
            return Err("unsupported_constraint");
        }
        if !class.canonical()
            && !self.session.policy.show_payloads
            && filter.identity.is_some()
            && filter.collection.as_deref() == Some("erasure_pending_redaction")
        {
            return Err("refused");
        }
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let ordering = if class == Class::Log { "rowid" } else { "write_cursor" };
        let mut statement=c.prepare(&format!("SELECT rowid,write_cursor FROM {} WHERE {ordering}>?1 ORDER BY {ordering} LIMIT 100000",class.table())).map_err(|_|"storage")?;
        let mut rows = statement.query([after]).map_err(|_| "storage")?;
        let mut items = vec![];
        let mut examined = 0;
        let mut last = after;
        let mut bytes = 0;
        let mut status = "complete";
        let mut unknown_classification = 0;
        let mut attribute_examined = 0;
        let mut attribute_budget_exhausted = false;
        let mut time_capped = false;
        let mut content_budget = 10 * 65536_usize;
        let mut inspected = 0;
        let mut skipped = std::collections::BTreeMap::<String, usize>::new();
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            if std::time::Instant::now() >= deadline {
                status = "partial";
                time_capped = true;
                break;
            }
            if examined >= 100000 {
                status = "partial";
                break;
            }
            examined += 1;
            let rowid: i64 = row.get(0).map_err(|_| "storage")?;
            let cursor: i64 = row.get(1).map_err(|_| "storage")?;
            let position = if class == Class::Log { rowid } else { cursor };
            let revision_rowid = if class.canonical() {
                c.query_row("SELECT rowid FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),cursor],|r|r.get(0)).optional().map_err(|_|"storage")?
            } else {
                None
            };
            let locator = Locator { class, rowid, cursor, revision_rowid };
            let (mut item, classification) = Self::summary(&c, &locator, self.session.policy)?;
            if classification.inactive && self.session.policy.no_show_inactive {
                last = position;
                continue;
            }
            if !classification.known {
                unknown_classification += 1;
                last = position;
                continue;
            }
            if classification.telemetry
                && !self.session.policy.show_payloads
                && filter.identity.is_some()
            {
                last = position;
                continue;
            }
            let matching = if class.canonical() {
                c.query_row(
                    &format!("SELECT CASE WHEN ?2 IS NULL THEN 1 WHEN octet_length(kind)!=octet_length(?2) THEN 0 ELSE COALESCE(kind=?2,0) END AND CASE WHEN ?3 IS NULL THEN 1 WHEN octet_length({role})!=octet_length(?3) THEN 0 ELSE COALESCE({role}=?3,0) END AND CASE WHEN ?4 IS NULL THEN 1 WHEN octet_length(source_id)!=octet_length(?4) THEN 0 ELSE COALESCE(source_id=?4,0) END AND CASE WHEN ?5 IS NULL THEN 1 WHEN octet_length(logical_id)!=octet_length(?5) THEN 0 ELSE COALESCE(logical_id=?5,0) END FROM {} WHERE rowid=?1",class.table(),role=if class==Class::Node{"row_kind"}else{"NULL"}),
                    params![rowid,filter.kind,filter.role,filter.source,filter.identity],
                    |r|r.get::<_,bool>(0),
                )
            } else {
                c.query_row(
                    &format!("SELECT CASE WHEN ?2 IS NULL THEN 1 WHEN octet_length(collection_name)!=octet_length(?2) THEN 0 ELSE COALESCE(collection_name=?2,0) END AND CASE WHEN ?3 IS NULL THEN 1 WHEN octet_length(record_key)!=octet_length(?3) THEN 0 ELSE COALESCE(record_key=?3,0) END FROM {} WHERE rowid=?1",class.table()),
                    params![rowid,filter.collection,filter.identity],
                    |r|r.get::<_,bool>(0),
                )
            }.map_err(|_| "storage")?;
            if !matching {
                last = position;
                continue;
            }
            if let Some(name) = &filter.attribute {
                match Self::attribute_matches(
                    &c,
                    cursor,
                    name,
                    &mut examined,
                    &mut attribute_examined,
                ) {
                    Ok(true) => {}
                    Ok(false) => {
                        last = position;
                        continue;
                    }
                    Err("budget_exhausted") => {
                        status = "partial";
                        attribute_budget_exhausted = true;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            if items.len() == limit {
                status = "partial";
                break;
            }
            item["payload_status"] = json!(if self.session.policy.permits(classification) {
                "available"
            } else {
                "refused"
            });
            if classification.telemetry && !self.session.policy.show_payloads {
                item["record_key"] = Value::Null;
            }
            if class.canonical() && !sample && self.session.policy.permits(classification) {
                item["display_preview"] =
                    Self::display_preview(&c, &locator, deadline, &mut content_budget)?;
            }
            if sample {
                let observation = if self.session.policy.permits(classification) {
                    Self::sample_fields(&c, &locator, deadline, &mut content_budget)?
                } else {
                    json!({"state":"refused","inspected":false,"names":[],"root_kind":null,"complete_for_this_record":false})
                };
                if observation["inspected"] == true {
                    inspected += 1;
                } else {
                    *skipped
                        .entry(observation["state"].as_str().unwrap_or("unknown").to_string())
                        .or_default() += 1;
                }
                item["field_observation"] = observation;
            }
            let size = serde_json::to_vec(&item).map_err(|_| "storage")?.len() + 100;
            if bytes + size > RESPONSE - 16384 {
                status = "partial";
                break;
            }
            bytes += size;
            last = position;
            item["handle"] = json!(self.mint(locator));
            items.push(item);
        }
        if examined >= 100000 {
            status = "partial";
        }
        let continuation = if status == "partial" && !attribute_budget_exhausted {
            Some(last.to_string())
        } else {
            None
        };
        if unknown_classification > 0 {
            status = "partial";
        }
        Ok(
            json!({"status":status,"items":items,"context":Self::finished(context),"scope":{"class":class,"filter":filter,"identity_lookup_excluded_namespaces":if !class.canonical()&&!self.session.policy.show_payloads&&filter.identity.is_some(){vec!["erasure_pending_redaction"]}else{vec![]},"sample":if sample{Some(json!({"selection":"first_matching_retained_records","record_limit":10,"field_limit":20,"content_per_record_bytes":65536,"content_bytes_inspected":10*65536-content_budget,"payloads_inspected":inspected,"payloads_skipped":skipped,"absence_proof":false,"meaning":"observed content fields; not a registered schema"}))}else{None},"policy":self.session.policy,"ordering":if class==Class::Log{"entry_id_ascending"}else{"write_cursor_ascending"},"limit":limit,"examined":examined,"attribute_examined":attribute_examined,"attribute_constraint_meaning":if filter.attribute.is_some(){Some("stored_scalar_materialization_presence; no body-field absence or readiness proof")}else{None},"unknown_classification":unknown_classification,"returned":items.len(),"continuation":continuation,"work_deadline_millis":WORK_TIME.as_millis(),"reason":if time_capped{Some("operation_work_deadline")}else if attribute_budget_exhausted{Some("attribute_scan_budget_exhausted")}else if unknown_classification>0{Some("unknown_classification")}else if status=="partial"{Some("page_or_work_or_byte_cap")}else{None}}}),
        )
    }
    pub(crate) fn check_many(
        &mut self,
        handle: Option<&str>,
        handles: &[String],
        metadata_handles: &[String],
    ) -> Result<Value, &'static str> {
        let count = handles.len() + metadata_handles.len() + usize::from(handle.is_some());
        if count > 201 {
            return Err("invalid_range");
        }
        let deadline = std::time::Instant::now() + WORK_TIME;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut remaining = 100000;
        for (handle, content) in handle
            .into_iter()
            .chain(handles.iter().map(String::as_str))
            .map(|h| (h, true))
            .chain(metadata_handles.iter().map(|h| (h.as_str(), false)))
        {
            if std::time::Instant::now() >= deadline {
                return Err("budget_exhausted");
            }
            let attribute = self.attributes.get(handle);
            let locator = match attribute {
                Some(attribute) => &attribute.origin,
                None => self.handles.get(handle).ok_or("reference_expired")?,
            };
            let (_, classification) = Self::summary_fields_budget(
                &c,
                locator,
                if locator.class.canonical() {
                    &["kind", "logical_id", "source_id"]
                } else {
                    &["collection_name"]
                },
                None,
                &mut remaining,
            )?;
            if !classification.known
                || (classification.inactive && self.session.policy.no_show_inactive)
                || (content && !self.session.policy.permits(classification))
            {
                return Err("refused");
            }
            if let Some(attribute) = attribute {
                if context["visibility_generation"].as_str()
                    != Some(attribute.visibility.0.as_str())
                    || context["visibility_nonce"].as_str() != Some(attribute.visibility.1.as_str())
                    || context["projection_generation"].as_str()
                        != Some(attribute.generation.as_str())
                {
                    return Err("reference_expired");
                }
                let exists: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM canonical_attributes WHERE rowid=?1 AND write_cursor=?2 AND attr_name=?3)", params![attribute.rowid, locator.cursor, attribute.name], |r|r.get(0)).map_err(|_|"storage")?;
                if !exists {
                    return Err("reference_expired");
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err("budget_exhausted");
        }
        Ok(
            json!({"status":"complete","context":Self::finished(context),"scope":{"validated":count,"examined":100000-remaining,"limit":201,"snapshot":"this_request_only"}}),
        )
    }
    pub(crate) fn catalog(&mut self) -> Result<Value, &'static str> {
        let deadline = std::time::Instant::now() + WORK_TIME;
        let mut time_capped = false;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut examined = 0_usize;
        let mut exact = true;
        let mut budget = CatalogBudget::default();
        let mut collections_complete = true;
        let mut projections_complete = true;
        let mut groups_complete = true;
        let mut operational_groups_complete = true;
        let mut counts = json!({});
        let mut groups = std::collections::BTreeMap::<String, usize>::new();
        let mut collections = vec![];
        let mut statement=c.prepare("SELECT rowid,kind,octet_length(name) FROM operational_collections ORDER BY name LIMIT 201").map_err(|_|"storage")?;
        let mut rows = statement.query([]).map_err(|_| "storage")?;
        while let Some(row) = rows.next().map_err(|_| "storage")? {
            if std::time::Instant::now() >= deadline {
                collections_complete = false;
                time_capped = true;
                exact = false;
                break;
            }
            examined += 1;
            if collections.len() == 200 {
                collections_complete = false;
                exact = false;
                break;
            }
            let rowid: i64 = row.get(0).map_err(|_| "storage")?;
            let mode: String = row.get(1).map_err(|_| "storage")?;
            let bytes: i64 = row.get(2).map_err(|_| "storage")?;
            let name = Self::preview(&c, "operational_collections", "name", rowid, 4096)?
                .ok_or("storage")?;
            let complete = bytes as usize == name.len();
            exact &= complete;
            collections_complete &= complete;
            let owned = complete
                && matches!(
                    name.as_str(),
                    "projection_failures"
                        | "erasure_pending_redaction"
                        | "purge_audit"
                        | "excise_source_audit"
                        | "excise_collection_audit"
                );
            let item = json!({"name":name,"name_bytes":bytes.to_string(),"name_complete":complete,"mode":mode,"engine_owned":owned,"payload_status":if name=="erasure_pending_redaction"&&!self.session.policy.show_payloads{"refused"}else{"available"}});
            if !budget.include(&item)? {
                collections_complete = false;
                exact = false;
                break;
            }
            collections.push(item);
        }
        let mut projections = vec![];
        let mut statement=c.prepare("SELECT rowid,vector_declared,octet_length(name),octet_length(roles),octet_length(fts_tokenizer),octet_length(entity_class),octet_length(vector_kinds) FROM _fathomdb_projection_registry ORDER BY name,entity_class LIMIT 201").map_err(|_|"storage")?;
        let mut rows = statement.query([]).map_err(|_| "storage")?;
        while let Some(row) = rows.next().map_err(|_| "storage")? {
            if std::time::Instant::now() >= deadline {
                projections_complete = false;
                time_capped = true;
                exact = false;
                break;
            }
            examined += 1;
            if projections.len() == 200 {
                projections_complete = false;
                exact = false;
                break;
            }
            let rowid: i64 = row.get(0).map_err(|_| "storage")?;
            let vector: bool = row.get(1).map_err(|_| "storage")?;
            let name = Self::preview(&c, "_fathomdb_projection_registry", "name", rowid, 4096)?
                .ok_or("storage")?;
            let roles = Self::preview(&c, "_fathomdb_projection_registry", "roles", rowid, 4096)?
                .ok_or("storage")?;
            let tokenizer =
                Self::preview(&c, "_fathomdb_projection_registry", "fts_tokenizer", rowid, 4096)?;
            let entity_class =
                Self::preview(&c, "_fathomdb_projection_registry", "entity_class", rowid, 4096)?
                    .ok_or("storage")?;
            let vector_kinds =
                Self::preview(&c, "_fathomdb_projection_registry", "vector_kinds", rowid, 4096)?;
            let name_bytes: i64 = row.get(2).map_err(|_| "storage")?;
            let roles_bytes: i64 = row.get(3).map_err(|_| "storage")?;
            let tokenizer_bytes: Option<i64> = row.get(4).map_err(|_| "storage")?;
            let entity_class_bytes: i64 = row.get(5).map_err(|_| "storage")?;
            let vector_kinds_bytes: Option<i64> = row.get(6).map_err(|_| "storage")?;
            let complete = name_bytes as usize == name.len();
            let declaration_complete = complete
                && roles_bytes as usize == roles.len()
                && tokenizer_bytes.map(|n| n as usize) == tokenizer.as_ref().map(|v| v.len())
                && entity_class_bytes as usize == entity_class.len()
                && vector_kinds_bytes.map(|n| n as usize) == vector_kinds.as_ref().map(|v| v.len());
            exact &= declaration_complete;
            projections_complete &= declaration_complete;
            let item = json!({"name":name,"name_bytes":name_bytes.to_string(),"name_complete":complete,"entity_class":entity_class,"roles":roles,"roles_bytes":roles_bytes.to_string(),"recorded_tokenizer":tokenizer,"tokenizer_bytes":tokenizer_bytes.map(|n|n.to_string()),"vector_declared":vector,"vector_kinds":vector_kinds,"vector_kinds_bytes":vector_kinds_bytes.map(|n|n.to_string()),"materialization":"unknown_until_record_check","runtime":if self.session.cached_cpu{"available_cached_cpu"}else{"model_unavailable"}});
            if !budget.include(&item)? {
                projections_complete = false;
                exact = false;
                break;
            }
            projections.push(item);
        }
        let metadata_examined = examined;
        for class in [Class::Node, Class::Edge, Class::Current, Class::Log] {
            let remaining = 100000_usize.saturating_sub(examined);
            let mut n = 0;
            let mut logical = std::collections::BTreeSet::new();
            let mut logical_known = true;
            let mut scanned = 0;
            let mut unknown_classification = 0;
            let mut category_capped = false;
            let mut statement = c
                .prepare(&format!(
                    "SELECT rowid,write_cursor FROM {} ORDER BY {} LIMIT ?1",
                    class.table(),
                    if class == Class::Log { "rowid" } else { "write_cursor" }
                ))
                .map_err(|_| "storage")?;
            let mut rows = statement.query([remaining as i64]).map_err(|_| "storage")?;
            while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                if std::time::Instant::now() >= deadline {
                    time_capped = true;
                    category_capped = true;
                    exact = false;
                    break;
                }
                examined += 1;
                scanned += 1;
                let rowid: i64 = row.get(0).map_err(|_| "storage")?;
                let cursor: i64 = row.get(1).map_err(|_| "storage")?;
                let revision_rowid = if class.canonical() {
                    c.query_row("SELECT rowid FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),cursor],|r|r.get(0)).optional().map_err(|_|"storage")?
                } else {
                    None
                };
                let (record, classification) =
                    Self::inventory_summary(&c, &Locator { class, rowid, cursor, revision_rowid })?;
                if !classification.known {
                    unknown_classification += 1;
                    groups_complete = false;
                    if !class.canonical() {
                        operational_groups_complete = false;
                    }
                    logical_known = false;
                    continue;
                }
                if classification.inactive && self.session.policy.no_show_inactive {
                    continue;
                }
                n += 1;
                if let Some(id) = record["logical_id"].as_str() {
                    if record["logical_id_bytes"].as_u64().unwrap_or(0) > 4096 {
                        logical_known = false;
                    } else {
                        logical.insert(id.to_string());
                    }
                }
                let group = record[if class.canonical() { "kind" } else { "collection_name" }]
                    .as_str()
                    .unwrap_or("unknown");
                let group_column = if class.canonical() { "kind" } else { "collection_name" };
                if record[format!("{group_column}_bytes")].as_u64() != Some(group.len() as u64) {
                    exact = false;
                    groups_complete = false;
                    if !class.canonical() {
                        operational_groups_complete = false;
                    }
                    continue;
                }
                let key = format!("{}:{group}", class.label());
                if let Some(count) = groups.get_mut(&key) {
                    *count += 1;
                } else if groups.len() < 200 && budget.include(&json!({key.clone():1}))? {
                    groups.insert(key, 1);
                } else {
                    exact = false;
                    groups_complete = false;
                    if !class.canonical() {
                        operational_groups_complete = false;
                    }
                }
            }
            let category_exact = scanned < remaining
                && unknown_classification == 0
                && !category_capped
                && !time_capped;
            if !category_exact {
                exact = false;
                groups_complete = false;
                if !class.canonical() {
                    operational_groups_complete = false;
                }
            }
            let unit = match class {
                Class::Node | Class::Edge => "revisions",
                Class::Current => "values",
                Class::Log => "entries",
            };
            counts[class.label()] = json!({unit:n.to_string(),"logical_identities":if class.canonical()&&logical_known{Some(logical.len().to_string())}else{None},"exact":category_exact,"unknown_classification":unknown_classification,"examined":scanned});
        }
        let remaining = 100000_usize.saturating_sub(examined);
        let mut projected = 0;
        let mut projected_unknown = 0;
        let mut scanned = 0;
        let mut statement=c.prepare("SELECT a.write_cursor,n.rowid,r.rowid FROM canonical_attributes a JOIN canonical_nodes n ON n.write_cursor=a.write_cursor LEFT JOIN _fathomdb_artifact_revisions r ON r.artifact_class='node' AND r.write_cursor=a.write_cursor ORDER BY a.rowid LIMIT ?1").map_err(|_|"storage")?;
        let mut rows = statement.query([remaining as i64]).map_err(|_| "storage")?;
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            if std::time::Instant::now() >= deadline {
                time_capped = true;
                exact = false;
                break;
            }
            examined += 1;
            scanned += 1;
            let locator = Locator {
                class: Class::Node,
                cursor: row.get(0).map_err(|_| "storage")?,
                rowid: row.get(1).map_err(|_| "storage")?,
                revision_rowid: row.get(2).map_err(|_| "storage")?,
            };
            let (_, policy) = Self::inventory_summary(&c, &locator)?;
            if !policy.known {
                projected_unknown += 1;
                exact = false;
            }
            if policy.known && !(policy.inactive && self.session.policy.no_show_inactive) {
                projected += 1;
            }
        }
        if scanned >= remaining {
            exact = false;
        }
        let attr_exact = scanned < remaining && projected_unknown == 0 && !time_capped;
        let cpv_remaining = 100000_usize.saturating_sub(examined);
        let mut cpv_entries = 0_usize;
        let mut cpv_unknown = 0_usize;
        let mut cpv_scanned = 0_usize;
        let mut statement = c
            .prepare(
                "SELECT p.entity_class,p.write_cursor,CASE WHEN p.entity_class='node' THEN n.rowid ELSE e.rowid END,r.rowid \
                 FROM canonical_property_values p \
                 LEFT JOIN canonical_nodes n ON p.entity_class='node' AND n.write_cursor=p.write_cursor \
                 LEFT JOIN canonical_edges e ON p.entity_class='edge' AND e.write_cursor=p.write_cursor \
                 LEFT JOIN _fathomdb_artifact_revisions r ON r.artifact_class=p.entity_class AND r.write_cursor=p.write_cursor \
                 WHERE (p.entity_class='node' AND n.rowid IS NOT NULL) OR (p.entity_class='edge' AND e.rowid IS NOT NULL) \
                 ORDER BY p.entity_class,p.write_cursor,p.attr_name LIMIT ?1",
            )
            .map_err(|_| "storage")?;
        let mut rows = statement.query([cpv_remaining as i64]).map_err(|_| "storage")?;
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            if std::time::Instant::now() >= deadline {
                time_capped = true;
                exact = false;
                break;
            }
            examined += 1;
            cpv_scanned += 1;
            let entity_class: String = row.get(0).map_err(|_| "storage")?;
            let class = match entity_class.as_str() {
                "node" => Class::Node,
                "edge" => Class::Edge,
                _ => {
                    cpv_unknown += 1;
                    exact = false;
                    continue;
                }
            };
            let locator = Locator {
                class,
                cursor: row.get(1).map_err(|_| "storage")?,
                rowid: row.get(2).map_err(|_| "storage")?,
                revision_rowid: row.get(3).map_err(|_| "storage")?,
            };
            let (_, policy) = Self::inventory_summary(&c, &locator)?;
            if !policy.known {
                cpv_unknown += 1;
                exact = false;
            }
            if policy.known && !(policy.inactive && self.session.policy.no_show_inactive) {
                cpv_entries += 1;
            }
        }
        if cpv_scanned >= cpv_remaining {
            exact = false;
        }
        let cpv_exact = cpv_scanned < cpv_remaining && cpv_unknown == 0 && !time_capped;
        counts["projection"] = json!({"attribute_entries":projected.to_string(),"property_value_entries":cpv_entries.to_string(),"property_value_examined":cpv_scanned,"property_value_unknown_classification":cpv_unknown,"property_value_exact":cpv_exact,"examined":scanned,"unknown_classification":projected_unknown,"exact":attr_exact&&cpv_exact,"scope":"verified canonical attribute and property-value origins; other representations separately inspected"});
        let ts_remaining = 100000_usize.saturating_sub(examined);
        let mut ts_scanned = 0_usize;
        let mut statement = c
            .prepare("SELECT write_cursor FROM _fathomdb_commit_timestamps ORDER BY write_cursor LIMIT ?1")
            .map_err(|_| "storage")?;
        let mut rows = statement.query([ts_remaining as i64]).map_err(|_| "storage")?;
        while rows.next().map_err(|_| "budget_exhausted")?.is_some() {
            if std::time::Instant::now() >= deadline {
                time_capped = true;
                exact = false;
                break;
            }
            examined += 1;
            ts_scanned += 1;
        }
        if ts_scanned >= ts_remaining {
            exact = false;
        }
        let ts_exact = ts_scanned < ts_remaining && !time_capped;
        counts["commit_timestamps"] =
            json!({"entries":ts_scanned.to_string(),"examined":ts_scanned,"exact":ts_exact});
        for key in groups.keys() {
            if let Some((class, name)) = key.split_once(':') {
                if matches!(class, "current" | "log")
                    && !collections.iter().any(|c| c["name"].as_str() == Some(name))
                {
                    if collections.len() >= 200 {
                        collections_complete = false;
                        exact = false;
                        break;
                    }
                    let owned = matches!(
                        name,
                        "projection_failures"
                            | "erasure_pending_redaction"
                            | "purge_audit"
                            | "excise_source_audit"
                            | "excise_collection_audit"
                    );
                    let item = json!({"name":name,"mode":if class=="log"{"append_only_log"}else{"latest_state"},"registration":"observed_unregistered_namespace","mode_evidence":"physical_current_layout","engine_owned":owned,"payload_status":if name=="erasure_pending_redaction"&&!self.session.policy.show_payloads{"refused"}else{"available"}});
                    if !budget.include(&item)? {
                        collections_complete = false;
                        exact = false;
                        break;
                    }
                    collections.push(item);
                }
            }
        }
        collections_complete &= operational_groups_complete
            && counts["current"]["exact"] == true
            && counts["log"]["exact"] == true;
        exact &= collections_complete && projections_complete && groups_complete;
        let result = json!({"status":if exact{"complete"}else{"partial"},"database":{"identity":self.session.database_id,"location":self.session.path,"schema":fathomdb_schema::SCHEMA_VERSION,"mode":"owned_engine","model_runtime":if self.session.cached_cpu{"available_cached_cpu"}else{"unavailable_no_model_loaded"},"model_support":if cfg!(feature="cached-cpu"){"compiled_cached_cpu"}else{"not_compiled"}},"context":Self::finished(context),"counts":counts,"groups":groups,"collections":collections,"projections":projections,"scope":{"policy":self.session.policy,"exact":exact,"examined":examined,"metadata_examined":metadata_examined,"work_deadline_millis":WORK_TIME.as_millis(),"time_capped":time_capped,"reason":if time_capped{Some("operation_work_deadline")}else if budget.partial{Some("shared_item_or_byte_cap")}else{None},"collections_complete":collections_complete,"projections_complete":projections_complete,"groups_complete":groups_complete,"returned_metadata_and_groups":budget.returned,"metadata_or_group_truncated":budget.partial||!collections_complete||!projections_complete||!groups_complete,"metadata_bytes_with_envelope_reserve":budget.bytes,"reserved_envelope_bytes":65536,"response_byte_cap":RESPONSE,"response_item_cap":200,"item_scope":"collection/declaration/group entries; fixed envelope and count facts separately bounded","row_cap":100000,"group_limit":200,"ordering":"declarations_then_class_keysets","observed_fields":"use bounded record tree; no exhaustive schema inference"}});
        if serde_json::to_vec(&result).map_err(|_| "storage")?.len() > RESPONSE {
            return Err("budget_exhausted");
        }
        Ok(result)
    }
    fn field<'a>(
        locator: &Locator,
        field: &'a str,
    ) -> Result<(&'static str, &'a str, i64), &'static str> {
        let class = locator.class;
        if field == "revision_id" && class.canonical() {
            return Ok((
                "_fathomdb_artifact_revisions",
                "revision_id",
                locator.revision_rowid.ok_or("missing")?,
            ));
        }
        let column = match field {
            "body" => class.body(),
            "properties" if class.canonical() => "properties",
            "structural_role" if class == Class::Node => "row_kind",
            "entry_id" if class == Class::Log => "rowid",
            _ if class.fields().contains(&field) => field,
            _ => return Err("unsupported_field"),
        };
        Ok((class.table(), column, locator.rowid))
    }
    pub(crate) fn chunk(
        &mut self,
        handle: &str,
        field: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Value, &'static str> {
        if limit == 0 || limit > CHUNK || offset > i64::MAX as usize - CHUNK {
            return Err("invalid_range");
        }
        if self.attributes.contains_key(handle) {
            return self.attribute_chunk(handle, field, offset, limit);
        }
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (_, class) = Self::summary(&c, &locator, self.session.policy)?;
        if !self.session.policy.permits(class) {
            return Err("refused");
        }
        let (table, column, rowid) = Self::field(&locator, field)?;
        let value = crate::window::Column::open(&c, table, column, rowid)?;
        let total = value.len();
        let bytes = value.read(offset, limit)?;
        let Some(total) = total else {
            return Ok(
                json!({"status":"complete","state":"null","text":"","continuation":null,"context":Self::finished(context),"offset":offset.to_string()}),
            );
        };
        if offset > total {
            return Err("invalid_range");
        }
        let (text, n) = utf8_prefix(bytes.as_deref().unwrap_or_default(), limit)?;
        if n == 0 && offset < total {
            return Err("invalid_utf8_boundary");
        }
        Ok(
            json!({"status":"complete","state":if total==0{"empty"}else{"value"},"text":text,"offset":offset.to_string(),"total_bytes":total.to_string(),"continuation":if offset+n<total{Some((offset+n).to_string())}else{None},"context":Self::finished(context)}),
        )
    }
    #[allow(dead_code)]
    pub(crate) fn tree(
        &mut self,
        handle: &str,
        start: usize,
        after: Option<usize>,
    ) -> Result<Value, &'static str> {
        self.tree_field(handle, None, start, after)
    }
    pub(crate) fn tree_field(
        &mut self,
        handle: &str,
        field: Option<&str>,
        start: usize,
        after: Option<usize>,
    ) -> Result<Value, &'static str> {
        let selected = match field {
            None | Some("body") => "body",
            Some("properties") => "properties",
            Some(_) => return Err("unsupported_field"),
        };
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let (table, column, rowid) = Self::field(&locator, selected)?;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (_, class) = Self::summary(&c, &locator, self.session.policy)?;
        if !self.session.policy.permits(class) {
            return Err("refused");
        }
        let value = crate::window::Column::open(&c, table, column, rowid)?;
        let total = value.len().ok_or("null")?;
        let mut result = crate::tree::children(
            |offset, limit| value.read(offset, limit)?.ok_or("null"),
            total,
            start,
            after,
        )?;
        result["field"] = json!(selected);
        result["context"] = Self::finished(context);
        Ok(result)
    }
    pub(crate) fn associations(&mut self, handle: &str) -> Result<Value, &'static str> {
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (_, classification) = Self::summary(&c, &locator, self.session.policy)?;
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        let mut budget = AssociationBudget::default();
        let mut links = vec![];
        let mut attributes = vec![];
        let mut property_values = vec![];
        let mut projections = vec![];
        let mut attributes_complete = true;
        let mut property_values_complete = true;
        let mut projections_complete = true;
        if locator.class.canonical() {
            for (kind,sql) in [
                ("recorded_source","SELECT a.artifact_class,a.write_cursor,a.rowid FROM _fathomdb_artifact_revisions a JOIN _fathomdb_source_links l ON l.source_revision_id=a.revision_id WHERE l.artifact_revision_id=(SELECT revision_id FROM _fathomdb_artifact_revisions WHERE rowid=?1) ORDER BY a.rowid LIMIT 201"),
                ("recorded_derived","SELECT a.artifact_class,a.write_cursor,a.rowid FROM _fathomdb_artifact_revisions a JOIN _fathomdb_source_links l ON l.artifact_revision_id=a.revision_id WHERE l.source_revision_id=(SELECT revision_id FROM _fathomdb_artifact_revisions WHERE rowid=?1) AND l.artifact_revision_id!=l.source_revision_id ORDER BY a.rowid LIMIT 201")
            ] {
                let mut targets=vec![];let mut withheld=false;let mut truncated=false;let mut unavailable=false;let mut examined=0;
                if let Some(revision)=locator.revision_rowid {
                    let mut statement=c.prepare(sql).map_err(|_|"storage")?;let mut rows=statement.query([revision]).map_err(|_|"storage")?;
                    while let Some(row)=rows.next().map_err(|_|"budget_exhausted")? {
                        budget.examined+=1;examined+=1;if examined>200{truncated=true;break;}
                        let class:String=row.get(0).map_err(|_|"storage")?;
                        let class=match class.as_str(){"node"=>Class::Node,"edge"=>Class::Edge,_=>return Err("unknown_classification")};
                        let cursor:i64=row.get(1).map_err(|_|"storage")?;let revision_rowid=Some(row.get(2).map_err(|_|"storage")?);
                        let rowid:Option<i64>=c.query_row(&format!("SELECT rowid FROM {} WHERE write_cursor=?1",class.table()),[cursor],|r|r.get(0)).optional().map_err(|_|"storage")?;
                        if let Some(rowid)=rowid {
                            let target=Locator{class,rowid,cursor,revision_rowid};let(mut record,policy)=Self::summary(&c,&target,self.session.policy)?;
                            if self.session.policy.permits(policy){if !budget.include(&record)?{truncated=true;break;}record["handle"]=json!(self.mint(target));targets.push(record);}else{withheld=true;}
                        }else{unavailable=true;}
                    }
                }
                budget.partial|=truncated;
                let resolution=if withheld{"refused"}else if truncated||unavailable{"partial_or_unknown"}else if targets.len()==1{"unique"}else if targets.len()>1{"choices"}else{"unknown_or_unavailable"};
                links.push(json!({"kind":kind,"resolution":resolution,"cardinality":targets.len(),"cardinality_scope":"returned_visible_targets","targets":targets,"truncated":truncated,"unavailable":unavailable,"examined":examined}));
            }
            if locator.class == Class::Node {
                let mut statement=c.prepare("SELECT rowid,octet_length(attr_value) FROM canonical_attributes WHERE write_cursor=?1 ORDER BY rowid LIMIT 201").map_err(|_|"storage")?;
                let mut rows = statement.query([locator.cursor]).map_err(|_| "storage")?;
                while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                    budget.examined += 1;
                    let rowid: i64 = row.get(0).map_err(|_| "storage")?;
                    let bytes: Option<i64> = row.get(1).map_err(|_| "storage")?;
                    let name_value = crate::window::Column::open(
                        &c,
                        "canonical_attributes",
                        "attr_name",
                        rowid,
                    )?;
                    let exact_name = name_value.len().is_some_and(|n| n <= 65536);
                    let name_bytes = name_value.read(0, 65536)?.ok_or("storage")?;
                    let (name, _) = utf8_prefix(&name_bytes, name_bytes.len())?;
                    let name = name.to_string();
                    let value =
                        Self::preview(&c, "canonical_attributes", "attr_value", rowid, 4096)?;
                    let mut item = json!({"name":name.chars().take(4096).collect::<String>(),"handle":null,"reference_status":if exact_name{"exact_materialization"}else{"identity_budget_exhausted"},"value":value,"total_bytes":bytes.map(|n|n.to_string()),"origin":"this_exact_canonical_cursor","preview_limit":4096});
                    if !budget.include(&item)? {
                        attributes_complete = false;
                        break;
                    }
                    if exact_name {
                        item["handle"] = json!(self.mint_attribute(AttributeLocator {
                            origin: locator.clone(),
                            rowid,
                            name,
                            visibility: (
                                context["visibility_generation"]
                                    .as_str()
                                    .ok_or("attachment_expired")?
                                    .to_string(),
                                context["visibility_nonce"]
                                    .as_str()
                                    .ok_or("attachment_expired")?
                                    .to_string()
                            ),
                            generation: context["projection_generation"]
                                .as_str()
                                .ok_or("attachment_expired")?
                                .to_string()
                        }));
                    }
                    attributes.push(item);
                }
            }
            let entity_class = locator.class.label();
            let mut statement = c
                .prepare(
                    "SELECT substr(CAST(attr_name AS BLOB),1,16384),\
                     value_type,octet_length(text_val),typeof(num_val),bool_val,\
                     CASE WHEN value_type='string' THEN substr(CAST(text_val AS BLOB),1,16384) END,\
                     CASE WHEN value_type IN ('integer','float') THEN num_val END \
                     FROM canonical_property_values \
                     WHERE entity_class=?1 AND write_cursor=?2 \
                     ORDER BY attr_name LIMIT 201",
                )
                .map_err(|_| "storage")?;
            let mut rows =
                statement.query(params![entity_class, locator.cursor]).map_err(|_| "storage")?;
            while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                budget.examined += 1;
                let name_bytes: Vec<u8> = row.get(0).map_err(|_| "storage")?;
                let (name_prefix, _) = utf8_prefix(&name_bytes, name_bytes.len())?;
                let name: String = name_prefix.chars().take(4096).collect();
                let value_type: String = row.get(1).map_err(|_| "storage")?;
                let text_bytes: Option<i64> = row.get(2).map_err(|_| "storage")?;
                let num_type: String = row.get(3).map_err(|_| "storage")?;
                let bool_val: Option<i64> = row.get(4).map_err(|_| "storage")?;
                let value = match value_type.as_str() {
                    "string" => {
                        let bytes: Option<Vec<u8>> = row.get(5).map_err(|_| "storage")?;
                        bytes
                            .map(|bytes| {
                                utf8_prefix(&bytes, bytes.len())
                                    .map(|(text, _)| text.chars().take(4096).collect())
                            })
                            .transpose()?
                    }
                    "integer" => {
                        if num_type == "integer" {
                            let n: i64 = row.get(6).map_err(|_| "storage")?;
                            Some(n.to_string())
                        } else {
                            let f: f64 = row.get(6).map_err(|_| "storage")?;
                            Some(f.to_string())
                        }
                    }
                    "float" => {
                        let f: f64 = row.get(6).map_err(|_| "storage")?;
                        Some(f.to_string())
                    }
                    "boolean" => {
                        bool_val
                            .map(|b| if b != 0 { "true".to_string() } else { "false".to_string() })
                    }
                    _ => None,
                };
                let item = json!({"entity_class":entity_class,"name":name,"value_type":value_type,"value":value,"total_bytes":text_bytes.map(|n|n.to_string()),"origin":"this_exact_canonical_cursor","preview_limit":4096});
                if !budget.include(&item)? {
                    property_values_complete = false;
                    break;
                }
                property_values.push(item);
            }
            let mut statement = c
                .prepare(
                    "SELECT p.rowid,\
                     CASE WHEN ?2='node' AND octet_length(p.name)<=65536 THEN EXISTS(SELECT 1 FROM canonical_attributes a WHERE CASE WHEN octet_length(a.attr_name)=octet_length(p.name) THEN a.attr_name=p.name ELSE 0 END AND a.write_cursor=?1) END,\
                     CASE WHEN octet_length(p.name)<=65536 THEN EXISTS(SELECT 1 FROM canonical_property_values pv WHERE pv.entity_class=?2 AND pv.write_cursor=?1 AND CASE WHEN octet_length(pv.attr_name)=octet_length(p.name) THEN pv.attr_name=p.name ELSE 0 END) END,\
                     p.vector_declared \
                     FROM _fathomdb_projection_registry p \
                     WHERE p.entity_class=?2 \
                     ORDER BY p.name LIMIT 201",
                )
                .map_err(|_| "storage")?;
            let mut rows =
                statement.query(params![locator.cursor, entity_class]).map_err(|_| "storage")?;
            while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                budget.examined += 1;
                let rowid: i64 = row.get(0).map_err(|_| "storage")?;
                let name = Self::preview(&c, "_fathomdb_projection_registry", "name", rowid, 4096)?
                    .ok_or("storage")?;
                let roles =
                    Self::preview(&c, "_fathomdb_projection_registry", "roles", rowid, 4096)?
                        .ok_or("storage")?;
                let vector_kinds = Self::preview(
                    &c,
                    "_fathomdb_projection_registry",
                    "vector_kinds",
                    rowid,
                    4096,
                )?;
                let materialized: Option<bool> = row.get(1).map_err(|_| "storage")?;
                let materialized_property_value: Option<bool> =
                    row.get(2).map_err(|_| "storage")?;
                let vector: bool = row.get(3).map_err(|_| "storage")?;
                let item = json!({"name":name,"entity_class":entity_class,"roles":roles,"declared":true,"materialized_attribute":materialized,"materialized_property_value":materialized_property_value,"vector_declared":vector,"vector_kinds":vector_kinds,"query_runtime":if self.session.cached_cpu{"available_cached_cpu"}else{"unavailable_without_loaded_model"},"readiness":"unknown; declaration is not a ready proof"});
                if !budget.include(&item)? {
                    projections_complete = false;
                    break;
                }
                projections.push(item);
            }
        }
        Ok(
            json!({"status":if budget.partial{"partial"}else{"complete"},"links":links,"attributes":attributes,"property_values":property_values,"projections":projections,"attributes_complete":attributes_complete,"property_values_complete":property_values_complete,"projections_complete":projections_complete,"context":Self::finished(context),"scope":{"limit":200,"returned":budget.returned,"examined":budget.examined,"item_bytes_with_handle_reserve":budget.bytes,"reason":if budget.partial{Some("shared_item_or_byte_or_link_scan_cap")}else{None},"ordering":"recorded_links_then_attribute_rowid_then_property_value_name_then_declaration_name","policy":self.session.policy}}),
        )
    }
    fn export_destination(
        &self,
        destination: &std::path::Path,
    ) -> Result<std::path::PathBuf, &'static str> {
        if !destination.is_absolute()
            || destination.components().any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("invalid_destination");
        }
        let parent = destination
            .parent()
            .ok_or("invalid_destination")?
            .canonicalize()
            .map_err(|_| "invalid_destination")?;
        let destination = parent.join(destination.file_name().ok_or("invalid_destination")?);
        let product = self.session.path.as_os_str().to_string_lossy();
        if ["", ".lock", "-wal", "-shm", "-journal"]
            .iter()
            .any(|suffix| destination == std::path::Path::new(&format!("{product}{suffix}")))
        {
            return Err("invalid_destination");
        }
        let components = destination
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        if components.iter().any(|s| s == "huggingface")
            || components.windows(2).any(|w| w == ["fathomdb", "embedders"])
        {
            return Err("invalid_destination");
        }
        let current_cache =
            self.session.source_cache_hint.as_ref().and_then(|p| p.canonicalize().ok());
        if self.session.private_root.as_ref().is_some_and(|p| destination.starts_with(p))
            || self.session.source_cache.as_ref().is_some_and(|p| destination.starts_with(p))
            || current_cache.as_ref().is_some_and(|p| destination.starts_with(p))
        {
            return Err("invalid_destination");
        }
        Ok(destination)
    }
    pub(crate) fn authorize_export(
        &mut self,
        handle: &str,
        destination: &std::path::Path,
    ) -> Result<Value, &'static str> {
        use std::os::unix::fs::MetadataExt;
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let c = self.session.connection()?;
        let (_, classification) = Self::summary(&c, &locator, self.session.policy)?;
        if !classification.known {
            return Err("unknown_classification");
        }
        if !classification.erased {
            return Err("unavailable");
        }
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        let content = crate::window::Column::open(
            &c,
            locator.class.table(),
            locator.class.body(),
            locator.rowid,
        )?;
        if content.len().is_none() {
            return Err("unavailable");
        }
        let identity = crate::window::Column::open(
            &c,
            locator.class.table(),
            if locator.class.canonical() { "logical_id" } else { "record_key" },
            locator.rowid,
        )?;
        if identity.len().is_some_and(|n| n > CHUNK) {
            return Err("budget_exhausted");
        }
        let destination = self.export_destination(destination)?;
        let parent = destination.parent().ok_or("invalid_destination")?;
        let metadata = parent.metadata().map_err(|_| "invalid_destination")?;
        Ok(
            json!({"status":"complete","destination":destination,"parent_device":metadata.dev().to_string(),"parent_inode":metadata.ino().to_string()}),
        )
    }
    pub(crate) fn export_to(
        &mut self,
        handle: &str,
        destination: &std::path::Path,
    ) -> Result<Value, &'static str> {
        self.export_to_sink(handle, destination, None)
    }
    pub(crate) fn export_to_sink(
        &mut self,
        handle: &str,
        destination: &std::path::Path,
        sink: Option<std::fs::File>,
    ) -> Result<Value, &'static str> {
        use std::os::unix::fs::OpenOptionsExt;
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let c = self.session.connection()?;
        let (record, classification) = Self::summary(&c, &locator, self.session.policy)?;
        if !classification.known {
            return Err("unknown_classification");
        }
        if !classification.erased {
            return Err("unavailable");
        }
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        let context = self.context(&c)?;
        let identity_column = if locator.class.canonical() { "logical_id" } else { "record_key" };
        let identity_value =
            crate::window::Column::open(&c, locator.class.table(), identity_column, locator.rowid)?;
        if identity_value.len().is_some_and(|n| n > CHUNK) {
            return Err("budget_exhausted");
        }
        let identity = identity_value
            .read(0, CHUNK)?
            .map(|bytes| String::from_utf8(bytes).map_err(|_| "invalid_utf8"))
            .transpose()?;
        drop(identity_value);
        let content = crate::window::Column::open(
            &c,
            locator.class.table(),
            locator.class.body(),
            locator.rowid,
        )?;
        let total = content.len().ok_or("unavailable")?;
        drop(content);
        drop(c);
        let destination = self.export_destination(destination)?;
        let scope = crate::export::ExportScope {
            database: self.session.database_id.clone(),
            attachment: self.attachment.clone(),
            class: locator.class,
            cursor: locator.cursor.to_string(),
            revision: record["revision_id"].as_str().map(String::from),
            identity: if locator.class.canonical() { identity.clone() } else { None },
            collection: record["collection_name"].as_str().map(String::from),
            key: if locator.class.canonical() { None } else { identity },
            observation: json!({"initial_read":Self::finished(context),"snapshot":"separate_authorized_chunk_transactions"}),
        };
        let mut output = match sink {
            Some(file) => file,
            None => std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&destination)
                .map_err(|_| "destination_unavailable")?,
        };
        let policy = self.session.policy;
        let mut offset = 0;
        let mut empty = total == 0;
        let chunks = std::iter::from_fn(|| {
            if offset >= total && !empty {
                return None;
            }
            empty = false;
            let result = (|| {
                let c = self.session.connection()?;
                let (_, classification) = Self::summary(&c, &locator, policy)?;
                if !classification.known
                    || !classification.erased
                    || !policy.permits(classification)
                {
                    return Ok(crate::export::CheckedChunk {
                        bytes: vec![],
                        classification,
                        available: true,
                    });
                }
                let value = crate::window::Column::open(
                    &c,
                    locator.class.table(),
                    locator.class.body(),
                    locator.rowid,
                )?;
                let bytes = value.read(offset, CHUNK)?.ok_or("unavailable")?;
                if bytes.is_empty() && offset < total {
                    return Err("unavailable");
                }
                offset += bytes.len();
                Ok(crate::export::CheckedChunk { bytes, classification, available: true })
            })();
            Some(result.unwrap_or_else(|_| {
                offset = total;
                crate::export::CheckedChunk {
                    bytes: vec![],
                    classification: Classification {
                        inactive: false,
                        erased: true,
                        telemetry: false,
                        known: false,
                    },
                    available: false,
                }
            }))
        });
        let mut result = crate::export::stream(&mut output, &scope, policy, chunks, || false);
        if result.status == "complete" && output.sync_all().is_err() {
            result.status = "partial";
            result.error = Some("sync_failed");
        }
        Ok(
            json!({"status":result.status,"bytes_written":result.bytes_written.to_string(),"payload_bytes":result.payload_bytes.to_string(),"error":result.error,"scope":scope}),
        )
    }
    fn detail_facts(&self, representation: &str, total: Option<usize>) -> Value {
        json!({
            "storage":{"state":if total.is_none(){"null"}else{"present"},"representation":representation,"bytes":total.map(|n|n.to_string()),"source":"inspector_current_layout","scope":"exact_representation","observation":"detail_context"},
            "eligibility":{"inspection":"allowed","retrieval":"unknown","reason":"retrieval_not_evaluated_by_this_route","source":"inspector_current_layout","scope":"exact_representation","observation":"detail_context"},
            "readiness":{"representation_read":"ready","retrieval":"unknown","reason":"successful_exact_read_is_not_a_retrieval_readiness_proof","source":"inspector_current_layout","scope":"exact_representation","observation":"detail_context"},
            "runtime":{"record_inspection":"available","dense_query":if self.session.cached_cpu{"available"}else{"unavailable"},"reason":if self.session.cached_cpu{"production_default_sealed_cache_cpu"}else{"session_opened_with_embedder_none"},"scope":"inspector_session","source":"Session::attach"}
        })
    }
    pub(crate) fn detail(&mut self, handle: &str) -> Result<Value, &'static str> {
        self.detail_field(handle, "body")
    }
    pub(crate) fn detail_field(
        &mut self,
        handle: &str,
        field: &str,
    ) -> Result<Value, &'static str> {
        if self.attributes.contains_key(handle) {
            let origin = self.attributes.get(handle).ok_or("reference_expired")?.origin.clone();
            let part = self.attribute_chunk(handle, field, 0, 65536)?;
            let origin_handle = self.mint(origin);
            let mut record = part["attribute"].clone();
            record["handle"] = json!(handle);
            return Ok(
                json!({"status":"complete","record":record,"selected_field":field,"origin_reference":{"class":part["attribute"]["origin"]["class"],"handle":origin_handle},"context":part["context"],"facts":self.detail_facts("scalar_attribute",part["total_bytes"].as_str().and_then(|n|n.parse().ok())),"content":{"state":part["state"],"text":part["text"],"offset":"0","total_bytes":part["total_bytes"],"continuation":part["continuation"]}}),
            );
        }
        let locator = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        let (table, column, rowid) = Self::field(&locator, field)?;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (mut record, class) = Self::summary(&c, &locator, self.session.policy)?;
        if !self.session.policy.permits(class) {
            return Err("refused");
        }
        record["handle"] = json!(handle);
        if locator.class.canonical() {
            record["display_preview"] = Self::display_preview(
                &c,
                &locator,
                std::time::Instant::now() + WORK_TIME,
                &mut 8192,
            )?;
        }
        let value = crate::window::Column::open(&c, table, column, rowid)?;
        let total = value.len();
        let body = value.read(0, 65536)?;
        let (text, next) = utf8_prefix(body.as_deref().unwrap_or_default(), 65536)?;
        let state = if total.is_none() {
            "null"
        } else if total == Some(0) {
            "empty"
        } else {
            "value"
        };
        Ok(
            json!({"status":"complete","record":record,"selected_field":field,"context":Self::finished(context),"facts":self.detail_facts(if field!="body"{"metadata_field"}else if locator.class.canonical(){"canonical_document"}else{"operational_payload"},total),"content":{"state":state,"text":text,"offset":"0","total_bytes":total.map(|n|n.to_string()),"continuation":if next<total.unwrap_or(0){Some(next.to_string())}else{None}}}),
        )
    }
}
fn utf8_prefix(bytes: &[u8], limit: usize) -> Result<(String, usize), &'static str> {
    let end = limit.min(bytes.len());
    let n = match std::str::from_utf8(&bytes[..end]) {
        Ok(_) => end,
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => return Err("invalid_utf8_boundary"),
    };
    Ok((std::str::from_utf8(&bytes[..n]).map_err(|_| "invalid_utf8")?.to_string(), n))
}

#[cfg(test)]
#[path = "../tests/integration/eligibility.rs"]
mod eligibility_tests;

#[path = "lexical.rs"]
pub(crate) mod lexical;

#[path = "vector.rs"]
pub(crate) mod vector;

#[path = "graph.rs"]
pub(crate) mod graph;
