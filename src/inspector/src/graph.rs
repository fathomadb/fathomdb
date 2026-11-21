use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    Incoming,
    Outgoing,
    Both,
}
struct Budget {
    work: usize,
    bytes: usize,
    partial: bool,
    incident: usize,
    scanned: usize,
}
impl Budget {
    fn new() -> Self {
        Self { work: 1000, bytes: 16384, partial: false, incident: 0, scanned: 0 }
    }
    fn charge(&mut self) -> bool {
        if self.work == 0 {
            self.partial = true;
            false
        } else {
            self.work -= 1;
            true
        }
    }
    fn include(&mut self, value: &Value, count: usize) -> bool {
        let size = serde_json::to_vec(value).unwrap().len() + 128;
        if count >= 200 || self.bytes + size > RESPONSE - 16384 {
            self.partial = true;
            return false;
        }
        self.bytes += size;
        true
    }
}
fn key(locator: &Locator) -> String {
    format!("{}:{}", locator.class.label(), locator.cursor)
}
impl Reader {
    fn graph_locator(
        c: &crate::session::ReadConnection,
        class: Class,
        rowid: i64,
        cursor: i64,
    ) -> Result<Locator, &'static str> {
        let revision_rowid = c.query_row("SELECT rowid FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),cursor],|r|r.get(0)).optional().map_err(|_|"storage")?;
        Ok(Locator { class, rowid, cursor, revision_rowid })
    }
    fn graph_text(
        c: &crate::session::ReadConnection,
        locator: &Locator,
        field: &str,
    ) -> Result<Option<String>, &'static str> {
        let column = crate::window::Column::open(c, locator.class.table(), field, locator.rowid)?;
        if column.len().is_some_and(|n| n > 65536) {
            return Err("budget_exhausted");
        }
        column
            .read(0, 65536)?
            .map(|v| String::from_utf8(v).map_err(|_| "unsupported_value_encoding"))
            .transpose()
    }
    fn graph_record(
        &mut self,
        c: &crate::session::ReadConnection,
        locator: &Locator,
        budget: &mut Budget,
    ) -> Result<Option<Value>, &'static str> {
        if budget.work == 0 {
            budget.partial = true;
            return Ok(None);
        }
        let (mut record, classification) =
            match Self::summary_fields_budget(c, locator, locator.class.fields(), &mut budget.work)
            {
                Ok(v) => v,
                Err("budget_exhausted" | "unknown_classification") => {
                    budget.partial = true;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
        if !self.session.policy.permits(classification) {
            budget.partial = true;
            return Ok(None);
        }
        record["key"] = json!(key(locator));
        record["handle"] = json!(self.mint(locator.clone()));
        Ok(Some(record))
    }
    // There is no format37 index covering historical logical IDs. One explicit
    // scan bounds work and leaves cardinality unknown when coverage is incomplete.
    fn graph_scan(
        c: &crate::session::ReadConnection,
        class: Class,
        identities: &BTreeSet<String>,
        budget: &mut Budget,
    ) -> Result<(Vec<Locator>, bool), &'static str> {
        let deadline = std::time::Instant::now() + WORK_TIME;
        let mut statement = c.prepare(&format!("SELECT rowid,write_cursor,octet_length(logical_id),CASE WHEN octet_length(logical_id)<=65536 THEN logical_id END FROM {} ORDER BY rowid LIMIT 100001",class.table())).map_err(|_|"storage")?;
        let mut rows = statement.query([]).map_err(|_| "storage")?;
        let mut keys = 0usize;
        let mut targets = vec![];
        let mut complete = true;
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            if budget.scanned == 100000 || std::time::Instant::now() >= deadline {
                complete = false;
                break;
            }
            budget.scanned += 1;
            let size: Option<usize> = row.get(2).map_err(|_| "storage")?;
            keys = keys.saturating_add(size.unwrap_or(0));
            if keys > 16 * 1024 * 1024 || size.is_some_and(|n| n > 65536) {
                complete = false;
                break;
            }
            let id: Option<String> = row.get(3).map_err(|_| "storage")?;
            if id.as_ref().is_some_and(|id| identities.contains(id)) {
                if targets.len() == 200 {
                    complete = false;
                    break;
                }
                targets.push(Self::graph_locator(
                    c,
                    class,
                    row.get(0).map_err(|_| "storage")?,
                    row.get(1).map_err(|_| "storage")?,
                )?);
            }
        }
        budget.partial |= !complete;
        Ok((targets, complete))
    }
    pub(crate) fn graph(
        &mut self,
        handle: &str,
        direction: Direction,
        kind: Option<String>,
        depth: usize,
        expanded: Vec<String>,
    ) -> Result<Value, &'static str> {
        if !(1..=3).contains(&depth)
            || expanded.len() > 100
            || kind.as_ref().is_some_and(|s| s.len() > 65536)
        {
            return Err("invalid_range");
        }
        let root = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        if !root.class.canonical() {
            return Err("unsupported_constraint");
        }
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut budget = Budget::new();
        let (_, policy) =
            Self::summary_fields_budget(&c, &root, root.class.fields(), &mut budget.work)?;
        if !self.session.policy.permits(policy) {
            return Err("refused");
        }
        let mut expansions = BTreeSet::new();
        let mut expansion_locations = BTreeMap::new();
        for h in expanded {
            let l = self.handles.get(&h).ok_or("reference_expired")?;
            if l.class != Class::Node {
                return Err("unsupported_constraint");
            }
            let (_, policy) =
                Self::summary_fields_budget(&c, l, l.class.fields(), &mut budget.work)?;
            if !self.session.policy.permits(policy) {
                return Err("refused");
            }
            if let Some(id) = Self::graph_text(&c, l, "logical_id")? {
                expansions.insert(id.clone());
                expansion_locations.insert(key(l), (l.clone(), h, id));
            }
        }
        let mut edges: BTreeMap<String, (Value, String, String)> = BTreeMap::new();
        let mut nodes = BTreeMap::new();
        let mut exact_ids = BTreeMap::new();
        let mut identities = BTreeSet::new();
        let mut frontier = VecDeque::new();
        if root.class == Class::Node {
            if let Some(id) = Self::graph_text(&c, &root, "logical_id")? {
                identities.insert(id.clone());
                exact_ids.insert(key(&root), id.clone());
                frontier.push_back((id, 0));
            }
            let mut record = self.graph_record(&c, &root, &mut budget)?.ok_or("refused")?;
            record["handle"] = json!(handle);
            budget.include(&record, 0);
            nodes.insert(key(&root), record);
        } else {
            let record = self.graph_record(&c, &root, &mut budget)?.ok_or("refused")?;
            let from = Self::graph_text(&c, &root, "from_id")?.ok_or("unavailable")?;
            let to = Self::graph_text(&c, &root, "to_id")?.ok_or("unavailable")?;
            identities.extend([from.clone(), to.clone()]);
            frontier.extend([(from.clone(), 0), (to.clone(), 0)]);
            budget.include(&record, 0);
            edges.insert(key(&root), (record, from, to));
        }
        // Explicitly selected expansion anchors consume the same response budget before edges.
        for (k, (locator, owner_handle, id)) in expansion_locations {
            identities.insert(id.clone());
            frontier.push_back((id.clone(), 0));
            if nodes.contains_key(&k) {
                continue;
            }
            let mut record = self.graph_record(&c, &locator, &mut budget)?.ok_or("refused")?;
            record["handle"] = json!(owner_handle);
            if !budget.include(&record, nodes.len() + edges.len()) {
                return Err("budget_exhausted");
            }
            exact_ids.insert(k.clone(), id);
            nodes.insert(k, record);
        }
        let deadline = std::time::Instant::now() + WORK_TIME;
        let mut visited = BTreeSet::new();
        while let Some((id, hop)) = frontier.pop_front() {
            if hop >= depth || !expansions.contains(&id) || !visited.insert(id.clone()) {
                continue;
            }
            for column in match direction {
                Direction::Incoming => vec!["to_id"],
                Direction::Outgoing => vec!["from_id"],
                Direction::Both => vec!["from_id", "to_id"],
            } {
                let sql=format!("SELECT rowid,write_cursor FROM canonical_edges INDEXED BY canonical_edges_{column}_idx WHERE {column}=?1 ORDER BY rowid LIMIT ?2");
                let mut statement = c.prepare(&sql).map_err(|_| "storage")?;
                let mut rows = statement.query(params![id, budget.work]).map_err(|_| "storage")?;
                while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                    if std::time::Instant::now() >= deadline || !budget.charge() {
                        budget.partial = true;
                        break;
                    }
                    budget.incident += 1;
                    let locator = Self::graph_locator(
                        &c,
                        Class::Edge,
                        row.get(0).map_err(|_| "storage")?,
                        row.get(1).map_err(|_| "storage")?,
                    )?;
                    if edges.contains_key(&key(&locator)) {
                        continue;
                    }
                    if let Some(filter) = kind.as_deref() {
                        if Self::graph_text(&c, &locator, "kind")?.as_deref() != Some(filter) {
                            continue;
                        }
                    }
                    let Some(record) = self.graph_record(&c, &locator, &mut budget)? else {
                        continue;
                    };
                    let from = Self::graph_text(&c, &locator, "from_id")?.ok_or("unavailable")?;
                    let to = Self::graph_text(&c, &locator, "to_id")?.ok_or("unavailable")?;
                    if !budget.include(&record, nodes.len() + edges.len()) {
                        break;
                    }
                    identities.extend([from.clone(), to.clone()]);
                    if matches!(direction, Direction::Outgoing | Direction::Both) && from == id {
                        frontier.push_back((to.clone(), hop + 1));
                    }
                    if matches!(direction, Direction::Incoming | Direction::Both) && to == id {
                        frontier.push_back((from.clone(), hop + 1));
                    }
                    edges.insert(key(&locator), (record, from, to));
                }
                if budget.work == 0 || nodes.len() + edges.len() >= 200 {
                    budget.partial = true;
                    break;
                }
            }
            if budget.work == 0 || nodes.len() + edges.len() >= 200 {
                budget.partial = true;
                break;
            }
        }
        let (candidates, mut resolved) =
            Self::graph_scan(&c, Class::Node, &identities, &mut budget)?;
        for locator in candidates {
            if nodes.contains_key(&key(&locator)) {
                continue;
            }
            if let Some(record) = self.graph_record(&c, &locator, &mut budget)? {
                if !budget.include(&record, nodes.len() + edges.len()) {
                    resolved = false;
                    break;
                }
                if let Some(id) = Self::graph_text(&c, &locator, "logical_id")? {
                    exact_ids.insert(key(&locator), id);
                }
                nodes.insert(key(&locator), record);
            } else {
                resolved = false;
            }
        }
        let mut endpoints = BTreeMap::new();
        for id in identities {
            let targets = exact_ids
                .iter()
                .filter(|(key, exact)| **exact == id && nodes.contains_key(*key))
                .map(|(key, _)| json!(key))
                .collect::<Vec<_>>();
            let resolution = if !resolved {
                "unknown"
            } else {
                match targets.len() {
                    0 => "zero",
                    1 => "one",
                    _ => "many",
                }
            };
            let (preview, _) = utf8_prefix(id.as_bytes(), 4096)?;
            endpoints.insert(id.clone(),json!({"key":format!("endpoint:{}",endpoints.len()),"identity":preview,"identity_complete":id.len()<=4096,"resolution":resolution,"targets":targets,"meaning":"logical_endpoint; observed_retained_candidates_not_historical_membership"}));
        }
        let mut output = vec![];
        for (_, (mut edge, from, to)) in edges {
            edge["from"] = endpoints[&from].clone();
            edge["to"] = endpoints[&to].clone();
            output.push(edge);
        }
        let mut result = json!({"status":if budget.partial{"partial"}else{"complete"},"nodes":nodes.into_values().collect::<Vec<_>>(),"edges":output,"seed":{"handle":handle,"key":key(&root)},"context":Self::finished(context),"scope":{"route":"stored_adjacency","direction":direction,"kind":kind,"depth":depth,"depth_origin":"each_explicit_expansion_anchor","examined_links":budget.incident,"work_units":1000-budget.work,"examined_canonical_rows":budget.scanned,"object_cap":200,"examined_link_cap":1000,"row_cap":100000,"key_byte_cap":16*1024*1024,"response_byte_cap":RESPONSE,"layout":"presentation_only","absence_cause":"undetermined","endpoint_resolution_complete":resolved,"policy":self.session.policy}});
        while serde_json::to_vec(&result).map_err(|_| "storage")?.len() > RESPONSE {
            result["status"] = json!("partial");
            result["scope"]["endpoint_resolution_complete"] = json!(false);
            if let Some(edge) = result["edges"].as_array_mut().and_then(|v| v.pop()) {
                let _ = edge;
            } else {
                return Err("budget_exhausted");
            }
        }
        Ok(result)
    }
}

impl Reader {
    fn revision_locator(
        c: &crate::session::ReadConnection,
        revision: &str,
    ) -> Result<Option<Locator>, &'static str> {
        let row:Option<(String,i64,i64)>=c.query_row("SELECT artifact_class,write_cursor,rowid FROM _fathomdb_artifact_revisions WHERE revision_id=?1",[revision],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|"storage")?;
        let Some((class, cursor, revision_rowid)) = row else {
            return Ok(None);
        };
        let class = match class.as_str() {
            "node" => Class::Node,
            "edge" => Class::Edge,
            _ => return Err("storage"),
        };
        let rowid = c
            .query_row(
                &format!("SELECT rowid FROM {} WHERE write_cursor=?1", class.table()),
                [cursor],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "storage")?;
        Ok(rowid.map(|rowid| Locator {
            class,
            rowid,
            cursor,
            revision_rowid: Some(revision_rowid),
        }))
    }
    pub(crate) fn investigate(&mut self, handle: &str) -> Result<Value, &'static str> {
        let root = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        if !root.class.canonical() {
            return Err("unsupported_constraint");
        }
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut budget = Budget::new();
        let (mut record, policy) =
            Self::summary_fields_budget(&c, &root, root.class.fields(), &mut budget.work)?;
        if !self.session.policy.permits(policy) {
            return Err("refused");
        }
        record["handle"] = json!(handle);
        if !budget.include(&record, 0) {
            return Err("budget_exhausted");
        }
        let mut revisions = vec![];
        let mut links = vec![];
        let id = Self::graph_text(&c, &root, "logical_id")?;
        let mut history_complete;
        if let Some(id) = id {
            let (candidates, complete) =
                Self::graph_scan(&c, root.class, &BTreeSet::from([id]), &mut budget)?;
            history_complete = complete;
            for locator in candidates {
                if let Some(target) = self.graph_record(&c, &locator, &mut budget)? {
                    if !budget.include(&target, revisions.len() + links.len()) {
                        history_complete = false;
                        break;
                    }
                    revisions.push(target);
                } else {
                    history_complete = false;
                }
            }
        } else {
            history_complete = false;
        }
        if let Some(revision_rowid) = root.revision_rowid {
            let revision: String=c.query_row("SELECT revision_id FROM _fathomdb_artifact_revisions WHERE rowid=?1 AND octet_length(revision_id)<=65536",[revision_rowid],|r|r.get(0)).map_err(|_|"budget_exhausted")?;
            for (kind,sql) in [
                ("recorded_source","SELECT source_revision_id, NULL, NULL, locator_kind, start_byte, end_byte, hash_algorithm, hash_digest FROM _fathomdb_source_links WHERE artifact_revision_id=?1 AND source_revision_id!=artifact_revision_id LIMIT ?2"),
                ("recorded_derived","SELECT artifact_revision_id, NULL, NULL, locator_kind, start_byte, end_byte, hash_algorithm, hash_digest FROM _fathomdb_source_links INDEXED BY _fathomdb_source_links_source_derived_idx WHERE source_revision_id=?1 AND artifact_revision_id!=source_revision_id ORDER BY artifact_revision_id LIMIT ?2"),
                ("registered_dependency_to_source","SELECT l.source_revision_id, d.dependency_id, d.registered_dependency_generation, l.locator_kind, l.start_byte,l.end_byte,l.hash_algorithm,l.hash_digest FROM _fathomdb_source_dependencies d JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id WHERE d.derived_revision_id=?1 LIMIT ?2"),
                ("registered_dependency_to_dependents","SELECT l.artifact_revision_id, d.dependency_id, d.registered_dependency_generation, l.locator_kind,l.start_byte,l.end_byte,l.hash_algorithm,l.hash_digest FROM _fathomdb_source_links l INDEXED BY _fathomdb_source_links_source_derived_idx JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=l.artifact_revision_id WHERE l.source_revision_id=?1 ORDER BY l.artifact_revision_id LIMIT ?2"),
            ]{
                let (columns,from)=sql.strip_prefix("SELECT ").unwrap().split_once(" FROM ").unwrap();
                let columns=columns.split(',').map(str::trim).collect::<Vec<_>>();
                let sql=format!("SELECT CASE WHEN octet_length({target})<=65536 THEN {target} END,substr({dep},1,4096),{generation},{locator},{start},{end},{algorithm},substr({digest},1,4096),octet_length({dep}) FROM {from}",target=columns[0],dep=columns[1],generation=columns[2],locator=columns[3],start=columns[4],end=columns[5],algorithm=columns[6],digest=columns[7]);
                let mut statement=c.prepare(&sql).map_err(|_|"storage")?;
                let mut rows=statement.query(params![revision,budget.work]).map_err(|_|"storage")?;
                while let Some(row)=rows.next().map_err(|_|"budget_exhausted")?{
                    if !budget.charge(){break;}
                    budget.incident+=1;
                    let target_revision:Option<String>=row.get(0).map_err(|_|"storage")?;
                    let mut target=None;
                    if let Some(revision)=target_revision.as_deref(){if let Some(locator)=Self::revision_locator(&c,revision)?{target=self.graph_record(&c,&locator,&mut budget)?;}}
                    let metadata=if target.is_some(){json!({"dependency_id":row.get::<_,Option<String>>(1).map_err(|_|"storage")?,"registered_generation":row.get::<_,Option<i64>>(2).map_err(|_|"storage")?.map(|n|n.to_string()),"locator_kind":row.get::<_,String>(3).map_err(|_|"storage")?,"start_byte":row.get::<_,Option<i64>>(4).map_err(|_|"storage")?.map(|n|n.to_string()),"end_byte":row.get::<_,Option<i64>>(5).map_err(|_|"storage")?.map(|n|n.to_string()),"hash_algorithm":row.get::<_,String>(6).map_err(|_|"storage")?,"hash_digest":row.get::<_,String>(7).map_err(|_|"storage")?,"dependency_id_bytes":row.get::<_,Option<usize>>(8).map_err(|_|"storage")?})}else{Value::Null};
                    let link=json!({"kind":kind,"resolution":if target.is_some(){"exact_revision"}else{"unavailable_or_withheld"},"target":target,"metadata":metadata});
                    if !budget.include(&link,revisions.len()+links.len()){break;}
                    links.push(link);
                }
                if budget.work==0 || revisions.len()+links.len()>=200 {budget.partial=true;break;}
            }
        }
        Ok(
            json!({"status":if budget.partial{"partial"}else{"complete"},"record":record,"revisions":revisions,"links":links,"context":Self::finished(context),"scope":{"route":"retained_history_recorded_links","history_complete":history_complete,"revision_order":"retained_write_cursor; not_a_proven_causal_successor_chain","examined_links":budget.incident,"work_units":1000-budget.work,"examined_canonical_rows":budget.scanned,"item_cap":200,"work_cap":1000,"retrieval_absence_cause":"undetermined","policy":self.session.policy}}),
        )
    }
}

impl Reader {
    fn graph_admission(&mut self) -> Result<Value, &'static str> {
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let deadline = std::time::Instant::now() + WORK_TIME;
        let mut count = 0u64;
        let mut total = 0u64;
        let mut node_max = 0u64;
        let mut edge_max = 0u64;
        let mut source_max = 0u64;
        let mut key_max = 0u64;
        for (table, columns) in [
            ("canonical_nodes", &["body", "kind", "logical_id", "source_id"][..]),
            (
                "canonical_edges",
                &[
                    "body",
                    "kind",
                    "logical_id",
                    "source_id",
                    "from_id",
                    "to_id",
                    "extractor_model_id",
                ][..],
            ),
            (
                "_fathomdb_artifact_revisions",
                &["revision_id", "artifact_class", "artifact_role", "completeness"][..],
            ),
            (
                "_fathomdb_source_links",
                &[
                    "artifact_revision_id",
                    "source_id",
                    "source_version_id",
                    "source_revision_id",
                    "locator_kind",
                    "hash_algorithm",
                    "hash_digest",
                ][..],
            ),
        ] {
            let lengths = columns
                .iter()
                .map(|v| format!("COALESCE(octet_length({v}),0)"))
                .collect::<Vec<_>>()
                .join(",");
            let source = if table == "canonical_nodes" {
                ",EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions a WHERE a.artifact_class='node' AND a.write_cursor=canonical_nodes.write_cursor AND a.artifact_role='canonical_source')"
            } else {
                ""
            };
            let mut statement = c
                .prepare(&format!("SELECT {lengths}{source} FROM {table} LIMIT 100001"))
                .map_err(|_| "storage")?;
            let mut rows = statement.query([]).map_err(|_| "storage")?;
            while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
                count += 1;
                if count > 100000 || std::time::Instant::now() >= deadline {
                    return Err("budget_exhausted");
                }
                let mut metadata = 0u64;
                for index in 0..columns.len() {
                    let n: u64 = row.get(index).map_err(|_| "storage")?;
                    total = total.checked_add(n + 128).ok_or("budget_exhausted")?;
                    if index == 0 && table == "canonical_nodes" {
                        node_max = node_max.max(n);
                        if row.get::<_, bool>(columns.len()).map_err(|_| "storage")? {
                            source_max = source_max.max(n);
                        }
                    } else if index == 0 && table == "canonical_edges" {
                        edge_max = edge_max.max(n);
                    } else {
                        metadata = metadata.checked_add(n).ok_or("budget_exhausted")?;
                    }
                }
                key_max = key_max.max(metadata);
                if total > 32 * 1024 * 1024 {
                    return Err("budget_exhausted");
                }
            }
        }
        let charged =
            32 * 1024 * 1024 + node_max * 50 + edge_max * 50 + source_max * 100 + key_max * 1000;
        if charged > 64 * 1024 * 1024 {
            return Err("budget_exhausted");
        }
        Ok(
            json!({"scope":"before_native_call; separate_read","rows":count,"scanned_bytes":total,"charged_bytes":charged,"byte_cap":64*1024*1024,"source_hydration_multiplicity":100,"target_body_multiplicity":50,"terminal_body_multiplicity":50,"whole_payload_preload":false,"context":Self::finished(context)}),
        )
    }
    pub(crate) fn graph_serving(
        &mut self,
        handle: &str,
        direction: Direction,
        kind: Option<String>,
        depth: usize,
    ) -> Result<Value, &'static str> {
        use fathomdb_engine::{
            GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, ReadContextV1, ReadView,
            SearchFilter, TraversalDirection,
        };
        if !(1..=3).contains(&depth) || kind.as_ref().is_some_and(|s| s.len() > 4096) {
            return Err("invalid_range");
        }
        let seed = self.handles.get(handle).cloned().ok_or("reference_expired")?;
        if seed.class != Class::Node {
            return Err("unsupported_constraint");
        }
        let id = {
            let c = self.session.connection()?;
            let mut work = 1000;
            let (_, p) = Self::summary_fields_budget(&c, &seed, seed.class.fields(), &mut work)?;
            if !self.session.policy.permits(p) {
                return Err("refused");
            }
            Self::graph_text(&c, &seed, "logical_id")?.ok_or("unsupported_constraint")?
        };
        let admission = self.graph_admission()?;
        let frozen = self
            .session
            .engine
            .freeze_read_context(
                &ReadContextV1::new(ReadView::default(), SearchFilter::default())
                    .map_err(|_| "native_query_refused")?,
            )
            .map_err(|_| "native_query_refused")?;
        let started = crate::session::epoch_ms();
        let result = self
            .session
            .engine
            .graph_expand(&GraphExpandRequestV1 {
                schema_version: 1,
                seed: GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![fathomdb_engine::IdSpace::logical(id.clone())],
                },
                direction: match direction {
                    Direction::Incoming => TraversalDirection::Incoming,
                    Direction::Outgoing => TraversalDirection::Outgoing,
                    Direction::Both => TraversalDirection::Both,
                },
                edge_kinds: kind.clone().into_iter().collect(),
                target_kinds: vec![],
                context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
                max_depth: depth as u32,
                result_limit: 50,
                max_work_units: 1000,
                include_explanation: true,
                include_evidence: true,
            })
            .map_err(|_| "native_query_refused")?;
        let ended = crate::session::epoch_ms();
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut budget = Budget::new();
        let (_, seed_policy) =
            Self::summary_fields_budget(&c, &seed, seed.class.fields(), &mut budget.work)?;
        if !self.session.policy.permits(seed_policy) {
            return Err("refused");
        }
        let mut items = vec![];
        let mut withheld = 0;
        let evidence = result.evidence.as_ref().ok_or("native_evidence_unavailable")?;
        for (index, target) in result.targets.iter().enumerate() {
            let sidecar = evidence
                .entries
                .iter()
                .find(|v| v.target_index == index as u32)
                .ok_or("native_evidence_unavailable")?;
            let mut origins = vec![];
            let mut allowed = true;
            for logical in [&target.origin.seed_logical_id, &target.origin.predecessor_logical_id] {
                let row:Option<(i64,i64)>=c.query_row("SELECT rowid,write_cursor FROM canonical_nodes INDEXED BY canonical_nodes_logical_active_idx WHERE logical_id=?1 AND superseded_at IS NULL",[logical],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|"storage")?;
                let Some((rowid, cursor)) = row else {
                    allowed = false;
                    break;
                };
                let locator = Self::graph_locator(&c, Class::Node, rowid, cursor)?;
                if let Some(record) = self.graph_record(&c, &locator, &mut budget)? {
                    origins.push(record);
                } else {
                    allowed = false;
                    break;
                }
            }
            if !allowed {
                withheld += 1;
                continue;
            }
            let Some(target_locator) =
                Self::revision_locator(&c, sidecar.target_artifact_revision_id.as_str())?
            else {
                withheld += 1;
                continue;
            };
            let Some(edge_locator) =
                Self::revision_locator(&c, sidecar.terminal_edge_artifact_revision_id.as_str())?
            else {
                withheld += 1;
                continue;
            };
            let Some(target_record) = self.graph_record(&c, &target_locator, &mut budget)? else {
                withheld += 1;
                continue;
            };
            let Some(edge) = self.graph_record(&c, &edge_locator, &mut budget)? else {
                withheld += 1;
                continue;
            };
            if target_locator.cursor != i64::try_from(target.write_cursor).map_err(|_| "storage")? {
                return Err("native_evidence_unavailable");
            }
            let explanation = result
                .explanation
                .as_ref()
                .and_then(|e| e.per_target.iter().find(|v| v.target_index == index as u32));
            let (payload_text, payload_bytes) = utf8_prefix(target.body.as_bytes(), 4096)?;
            let item = json!({"payload":{"text":payload_text,"total_bytes":target.body.len().to_string(),"complete":payload_bytes==target.body.len(),"scope":"native_frozen_graph_expand"},"target":target_record,"terminal_edge":edge,"origin_records":origins,"evidence":{"target_revision":sidecar.target_artifact_revision_id.as_str(),"terminal_edge_revision":sidecar.terminal_edge_artifact_revision_id.as_str(),"target_reference":sidecar.target_evidence_ref.as_str(),"edge_reference":sidecar.terminal_edge_evidence_ref.as_str(),"hop_count":target.origin.hop_count,"terminal_direction":format!("{:?}",target.origin.terminal_direction),"lifecycle":explanation.map(|e|format!("{:?}",e.lifecycle_state)),"dependency":explanation.map(|e|format!("{:?}",e.dependency_state)),"scope":"actual_frozen_selected_artifacts"}});
            if !budget.include(&item, items.len() * 4) {
                break;
            }
            items.push(item);
        }
        let explanation = result.explanation.as_ref();
        let value = json!({"status":if budget.partial{"partial"}else{"complete"},"items":items,"admission":admission,"request":{"seed_kind":"explicit_logical; not_exact_historical_revision","seed_logical_id":id,"seed_reference":{"class":"node","key":key(&seed),"handle":handle},"direction":direction,"kind":kind,"depth":depth,"result_limit":50,"max_work_units":1000},"query_context":{"route":"engine_frozen_graph_expand","snapshot":"native_frozen_graph_expand","read_started_epoch_ms":started.to_string(),"read_ended_epoch_ms":ended.to_string(),"effective_valid_at":frozen.effective_valid_at.to_string(),"frozen_token":frozen.token,"work_units":result.work_units,"complete":result.complete,"read_mode":explanation.map(|e|format!("{:?}",e.read_mode)),"correlation_id":explanation.map(|e|&e.correlation_id),"projection_origin":explanation.map(|e|format!("{:?}",e.projection_origin)),"projection_readiness":explanation.map(|e|format!("{:?}",e.projection_readiness))},"context":Self::finished(context),"scope":{"route":"actual_serving_graph_evidence","native_returned":result.targets.len(),"returned":items.len(),"withheld":withheld,"hydration_snapshot":"separate_later_policy_read","absence_cause":"undetermined","rank":"not_applicable","policy":self.session.policy}});
        if serde_json::to_vec(&value).map_err(|_| "storage")?.len() > RESPONSE {
            return Err("budget_exhausted");
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "../tests/integration/graph.rs"]
mod tests;
