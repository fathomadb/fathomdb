use super::*;

impl Reader {
    pub(crate) fn indexed(
        &mut self,
        handle: &str,
        key: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<Value, &'static str> {
        if limit == 0 || limit > 65536 || (key.is_none() && (offset != 0 || limit > 4096)) {
            return Err("invalid_range");
        }
        let locator = self
            .attributes
            .get(handle)
            .map(|a| a.origin.clone())
            .or_else(|| self.handles.get(handle).cloned())
            .ok_or("reference_expired")?;
        if !locator.class.canonical() {
            return Err("unsupported_constraint");
        }
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (record, classification) = Self::summary(&c, &locator)?;
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        let origin_handle = if self.attributes.contains_key(handle) {
            self.mint(locator.clone())
        } else {
            handle.to_string()
        };
        let origin =
            json!({"handle":origin_handle,"record":record,"scope":"later_exact_stored_cursor"});
        let mut definitions = vec![];
        let table =
            if locator.class == Class::Node { "search_index" } else { "search_index_edges" };
        let mut rows = c
            .prepare(&format!(
                "SELECT rowid FROM {table} WHERE write_cursor=?1 ORDER BY rowid LIMIT 201"
            ))
            .map_err(|_| "storage")?;
        for row in
            rows.query_map([locator.cursor], |r| r.get::<_, i64>(0)).map_err(|_| "storage")?
        {
            definitions.push((
                "body".to_string(),
                table,
                "body",
                "c0",
                row.map_err(|_| "storage")?,
                None,
            ));
        }
        if locator.class == Class::Node {
            let mut rows = c.prepare("SELECT rowid FROM search_index_v2 WHERE write_cursor=?1 ORDER BY rowid LIMIT 201").map_err(|_|"storage")?;
            for row in
                rows.query_map([locator.cursor], |r| r.get::<_, i64>(0)).map_err(|_| "storage")?
            {
                let rowid = row.map_err(|_| "storage")?;
                for (name, column) in [("kind", "c0"), ("body", "c1"), ("status", "c2")] {
                    definitions.push((
                        format!("bm25f:{name}"),
                        "search_index_v2",
                        name,
                        column,
                        rowid,
                        None,
                    ));
                }
            }
            let mut rows=c.prepare("SELECT rowid,CASE WHEN octet_length(attr_name)<=4096 THEN attr_name END FROM property_search_index WHERE write_cursor=?1 ORDER BY rowid LIMIT 201").map_err(|_|"storage")?;
            for row in rows
                .query_map([locator.cursor], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
                })
                .map_err(|_| "storage")?
            {
                let (rowid, name) = row.map_err(|_| "storage")?;
                let name = name.ok_or("budget_exhausted")?;
                let configured:Option<String>=c.query_row("SELECT CASE WHEN octet_length(fts_tokenizer)<=4096 THEN fts_tokenizer END FROM _fathomdb_projection_registry WHERE name=?1",[&name],|r|r.get(0)).optional().map_err(|_|"storage")?.flatten();
                definitions.push((
                    format!("property:{name}"),
                    "property_search_index",
                    "attr_value",
                    "c0",
                    rowid,
                    configured,
                ));
            }
        }
        let total = definitions.len();
        let mut items = vec![];
        let mut bytes = 16384;
        let mut partial = false;
        for (name, table, field, column, rowid, configured) in definitions {
            if key.is_some_and(|key| key != name) {
                continue;
            }
            if items.len() == 200 {
                partial = true;
                break;
            }
            let ddl: Option<String> = c
                .query_row(
                    "SELECT substr(sql,1,8192) FROM sqlite_schema WHERE name=?1",
                    [table],
                    |r| r.get(0),
                )
                .map_err(|_| "storage")?;
            let shadow = format!("{table}_content");
            let value = crate::window::Column::open(&c, &shadow, column, rowid)?;
            let length = value.len();
            let window = value.read(offset, limit)?;
            let (text, n) = utf8_prefix(window.as_deref().unwrap_or_default(), limit)?;
            if n == 0 && length.is_some_and(|n| offset < n) {
                return Err("invalid_utf8_boundary");
            }
            let next = if offset + n < length.unwrap_or(0) {
                Some((offset + n).to_string())
            } else {
                None
            };
            let item = json!({"key":name,"index":table,"field":field,"text":text,"offset":offset.to_string(),"total_bytes":length.map(|n|n.to_string()),"continuation":next,"complete":offset==0&&next.is_none(),"state":if length.is_none(){"null"}else if length==Some(0){"empty"}else{"value"},"configured_tokenizer":configured,"effective_ddl":ddl,"effective_tokenizer":if ddl.as_deref().is_some_and(|v|v.contains("porter unicode61 remove_diacritics 2")){"porter unicode61 remove_diacritics 2"}else{"unknown; inspect actual DDL"},"tokenizer_source":"actual_selected_fts_ddl","origin":origin,"evidence":"stored_index_read; not query match evidence"});
            let size = serde_json::to_vec(&item).map_err(|_| "storage")?.len() + 1;
            if bytes + size > RESPONSE {
                partial = true;
                break;
            }
            bytes += size;
            items.push(item);
        }
        if key.is_some() && items.is_empty() {
            return Err("unavailable");
        }
        Self::lexical_bounded(
            json!({"status":if partial{"partial"}else{"complete"},"items":items,"context":Self::finished(context),"scope":{"representation_count_observed":total,"materialization":if total==0{"not_observed"}else{"present"},"retrieval_eligibility":"not_evaluated","origin":origin,"policy":self.session.policy,"limit":limit,"response_byte_cap":RESPONSE,"item_cap":200,"reason":if partial{Some("shared_item_or_byte_cap")}else{None}}}),
        )
    }
}

use fathomdb_engine::{EngineError, ReadView, SearchFilter, SnippetOptions, SoftFallbackBranch};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Route {
    Body,
    Property,
    Bm25f,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LexicalFilter {
    kind: Option<String>,
    source_type: Option<String>,
    status: Option<String>,
    created_after: Option<String>,
    #[serde(default)]
    attributes: Vec<(String, String)>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LexicalView {
    #[serde(default)]
    include_superseded: bool,
    #[serde(default)]
    include_inactive: bool,
    #[serde(default)]
    include_out_of_window: bool,
    valid_as_of: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Query {
    pub(crate) route: Route,
    pub(crate) query: String,
    pub(crate) property: Option<String>,
    #[serde(default)]
    pub(crate) filter: LexicalFilter,
    #[serde(default)]
    pub(crate) view: LexicalView,
    pub(crate) limit: usize,
}
fn engine_error(error: EngineError) -> &'static str {
    match error {
        EngineError::InvalidFilter { .. } => "unsupported_constraint",
        EngineError::InvalidArgument { .. } | EngineError::WriteValidation => "invalid_range",
        _ => "storage",
    }
}
impl Query {
    fn inputs(&self) -> Result<(Option<SearchFilter>, ReadView), &'static str> {
        let bounded = |v: &str| !v.is_empty() && v.len() <= 4096;
        if !bounded(&self.query)
            || self.query.trim().is_empty()
            || !(1..=100).contains(&self.limit)
            || self.filter.attributes.len() > 8
            || [
                self.property.as_deref(),
                self.filter.kind.as_deref(),
                self.filter.source_type.as_deref(),
                self.filter.status.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|v| !bounded(v))
            || self.filter.attributes.iter().any(|(a, b)| !bounded(a) || !bounded(b))
        {
            return Err("invalid_range");
        }
        if (self.route == Route::Property) != self.property.is_some() {
            return Err("unsupported_constraint");
        }
        let integer = |v: &Option<String>| {
            v.as_ref().map(|v| v.parse::<i64>().map_err(|_| "invalid_range")).transpose()
        };
        let mut filter = SearchFilter::default();
        filter.kind = self.filter.kind.clone();
        filter.source_type = self.filter.source_type.clone();
        filter.status = self.filter.status.clone();
        filter.created_after = integer(&self.filter.created_after)?;
        filter.attributes = self.filter.attributes.clone();
        let unfiltered = filter.kind.is_none()
            && filter.source_type.is_none()
            && filter.status.is_none()
            && filter.created_after.is_none()
            && filter.attributes.is_empty();
        let view = ReadView {
            include_superseded: self.view.include_superseded,
            include_inactive: self.view.include_inactive,
            include_out_of_window: self.view.include_out_of_window,
            valid_as_of: integer(&self.view.valid_as_of)?,
        };
        Ok((if unfiltered { None } else { Some(filter) }, view))
    }
}
impl Reader {
    pub(super) fn lexical_origins(
        &mut self,
        c: &crate::session::ReadConnection,
        class: Class,
        cursor: i64,
        remaining: &mut usize,
    ) -> Result<Vec<Value>, &'static str> {
        let mut statement = c
            .prepare(&format!("SELECT rowid FROM {} WHERE write_cursor=?1 LIMIT 3", class.table()))
            .map_err(|_| "storage")?;
        let mut origins = vec![];
        for row in statement.query_map([cursor], |r| r.get::<_, i64>(0)).map_err(|_| "storage")? {
            let rowid = row.map_err(|_| "storage")?;
            let revision_rowid=c.query_row("SELECT rowid FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),cursor],|r|r.get(0)).optional().map_err(|_|"storage")?;
            let locator = Locator { class, rowid, cursor, revision_rowid };
            let (record, policy) =
                Self::summary_fields_budget(c, &locator, class.fields(), remaining)?;
            if !self.session.policy.permits(policy) {
                return Err("refused");
            }
            let handle = self.mint(locator);
            origins.push(json!({"handle":handle,"record":record}));
        }
        Ok(origins)
    }
    pub(crate) fn lexical(&mut self, query: Query) -> Result<Value, &'static str> {
        let (filter, view) = query.inputs()?;
        if query.route == Route::Bm25f {
            return self.lexical_bm25f(&query, filter, view);
        }
        let admission = self.lexical_admission(&query)?;
        let started = crate::session::epoch_ms();
        let mut options = SnippetOptions::default();
        options.max_bytes = 512;
        options.highlights = true;
        let result = match query.route {
            Route::Body => self.session.engine.search_text_only_filtered_with_snippets(
                &query.query,
                filter,
                &view,
                query.limit,
                Some(&options),
            ),
            Route::Property => self.session.engine.search_projected_text_with_limit(
                &query.query,
                query.property.as_deref().ok_or("unsupported_constraint")?,
                filter,
                &view,
                query.limit,
            ),
            Route::Bm25f => unreachable!(),
        }
        .map_err(engine_error)?;
        let ended = crate::session::epoch_ms();
        let native_count = result.results.len();
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let metadata = Self::lexical_metadata(&c, &query)?;
        let query_context = json!({"route":query.route,"read_started_epoch_ms":started.to_string(),"read_ended_epoch_ms":ended.to_string(),"projection_cursor":result.projection_cursor.to_string(),"snapshot":"engine_query_reader_only; no authenticated snapshot token","later_checks":"context is a separate policy/origin read"});
        let mut items = vec![];
        let mut withheld = 0;
        let mut partial = false;
        let mut bytes = 16384;
        let mut remaining = 100000;
        for (rank, hit) in result.results.into_iter().enumerate() {
            let (class, branch) = match hit.branch {
                SoftFallbackBranch::Text => (Class::Node, "text"),
                SoftFallbackBranch::TextEdge => (Class::Edge, "text_edge"),
                _ => return Err("unsupported_value_encoding"),
            };
            let cursor =
                i64::try_from(hit.write_cursor).map_err(|_| "unsupported_value_encoding")?;
            let origins = match self.lexical_origins(&c, class, cursor, &mut remaining) {
                Ok(origins) => origins,
                Err("refused" | "unavailable" | "reference_expired") => {
                    withheld += 1;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if origins.is_empty() {
                withheld += 1;
                continue;
            }
            if !hit.score.is_finite() {
                return Err("unsupported_value_encoding");
            }
            let (text, n) = utf8_prefix(hit.body.as_bytes(), 4096)?;
            let snippet=result.snippets.as_ref().and_then(|v|v.get(rank)).map(|v|json!({"state":"available","text":v.text,"anchor":v.anchor.as_str(),"windows":v.windows,"spans":v.spans,"highlights":v.highlights,"cut":v.cut,"scope":"engine_query_reader","byte_ranges":"half_open_utf8","point_query_budget":20})).unwrap_or_else(||json!({"state":"unavailable","reason":"native_route_has_no_snippet_output"}));
            let identity_complete = hit.id.value.len() <= 4096;
            let item = json!({"rank":(rank+1).to_string(),"score":hit.score.to_string(),"class":class,"native_id":{"space":hit.id.space.as_str(),"value":if identity_complete{Some(hit.id.value)}else{None},"complete":identity_complete},"write_cursor":hit.write_cursor.to_string(),"representative_branch":branch,"kind":if hit.kind.len()<=4096{Some(hit.kind)}else{None},"source_id":hit.source_id.filter(|v|v.len()<=4096),"payload":{"text":text,"total_bytes":hit.body.len().to_string(),"complete":n==hit.body.len(),"scope":"engine_query_reader"},"snippet":snippet,"origin":{"state":if origins.len()==1{"exact"}else{"choices"},"targets":origins,"scope":"later_policy_and_exact_cursor_read","all_equal_body_contributors":"record_keyed_not_merged"}});
            let size = serde_json::to_vec(&item).map_err(|_| "storage")?.len() + 1;
            if bytes + size > RESPONSE {
                partial = true;
                break;
            }
            bytes += size;
            items.push(item);
        }
        Self::lexical_bounded(
            json!({"status":if partial{"partial"}else{"complete"},"items":items,"request":query,"admission":admission,"metadata":metadata,"query_context":query_context,"context":Self::finished(context),"scope":{"score_family":if query.route==Route::Body{"rrf"}else{"negated_sqlite_bm25"},"score_direction":"higher_better","native_returned":native_count,"returned":items.len(),"withheld":withheld,"rank_gaps":"original_native_ranks_preserved","absence":"not_proven_by_bounded_or_policy_filtered_results","limit":query.limit,"native_limit":query.limit,"policy":self.session.policy,"response_byte_cap":RESPONSE,"partial":partial}}),
        )
    }
}

impl Reader {
    fn lexical_bm25f(
        &mut self,
        query: &Query,
        filter: Option<SearchFilter>,
        view: ReadView,
    ) -> Result<Value, &'static str> {
        if filter.is_some() || view != ReadView::default() {
            return Err("unsupported_constraint");
        }
        let admission = self.lexical_admission(query)?;
        let started = crate::session::epoch_ms();
        let ranked = self
            .session
            .engine
            .bm25f_search(&query.query, &fathomdb_engine::Bm25fQueryPlan::default())
            .map_err(engine_error)?;
        let ended = crate::session::epoch_ms();
        let native_count = ranked.len();
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let metadata = Self::lexical_metadata(&c, query)?;
        let mut items = vec![];
        let mut bytes = 16384;
        let mut withheld = 0;
        let mut partial = false;
        let mut remaining = 100000;
        for (rank, (cursor, score)) in ranked.into_iter().take(query.limit).enumerate() {
            if !score.is_finite() {
                return Err("unsupported_value_encoding");
            }
            let cursor = i64::try_from(cursor).map_err(|_| "unsupported_value_encoding")?;
            let origins = match self.lexical_origins(&c, Class::Node, cursor, &mut remaining) {
                Ok(origins) => origins,
                Err("refused" | "unavailable" | "reference_expired") => {
                    withheld += 1;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if origins.is_empty() {
                withheld += 1;
                continue;
            }
            let rowid: i64 = c
                .query_row(
                    "SELECT rowid FROM canonical_nodes WHERE write_cursor=?1",
                    [cursor],
                    |r| r.get(0),
                )
                .map_err(|_| "unavailable")?;
            let value = crate::window::Column::open(&c, "canonical_nodes", "body", rowid)?;
            let body = value.read(0, 4096)?;
            let (text, n) = utf8_prefix(body.as_deref().unwrap_or_default(), 4096)?;
            let item = json!({"rank":(rank+1).to_string(),"score":score.to_string(),"class":"node","write_cursor":cursor.to_string(),"native_id":null,"representative_branch":"native_bm25f_substrate","payload":{"text":text,"total_bytes":value.len().map(|n|n.to_string()),"complete":Some(n)==value.len(),"scope":"later_exact_cursor_hydration"},"snippet":{"state":"unavailable","reason":"native_route_has_no_snippet_output"},"origin":{"state":if origins.len()==1{"exact"}else{"choices"},"targets":origins,"scope":"later_policy_and_exact_cursor_read","all_equal_body_contributors":"not_inferred"}});
            let size = serde_json::to_vec(&item).map_err(|_| "storage")?.len() + 1;
            if bytes + size > RESPONSE {
                partial = true;
                break;
            }
            bytes += size;
            items.push(item);
        }
        Self::lexical_bounded(
            json!({"status":if partial{"partial"}else{"complete"},"items":items,"request":query,"admission":admission,"metadata":metadata,"query_context":{"route":"bm25f","read_started_epoch_ms":started.to_string(),"read_ended_epoch_ms":ended.to_string(),"snapshot":"native_bm25f_no_snapshot_carrier","original_payload":"not_supplied","projection_cursor":null},"context":Self::finished(context),"scope":{"score_family":"native_bm25f","score_direction":"higher_better","native_returned":native_count,"returned":items.len(),"withheld":withheld,"rank_gaps":"original_native_ranks_preserved","limit":query.limit,"native_limit":null,"output_limit":"inspector_truncation_after_native_ranking","own_node_validity":"not_filtered_by_native_bm25f","eligibility":"native_active_current_and_dependency_source_eligibility","query_interpretation":"native_tokenizer_OR_terms","weights":{"kind":1,"body":1,"status":1},"b":"0.75","k1":"1.2","policy":self.session.policy,"response_byte_cap":RESPONSE,"partial":partial,"absence":"not_proven_by_bounded_or_policy_filtered_results"}}),
        )
    }
}

impl Reader {
    pub(super) fn lexical_admission(&mut self, query: &Query) -> Result<Value, &'static str> {
        const ROW_CAP: u64 = 100000;
        const BYTE_CAP: u64 = 64 * 1024 * 1024;
        const VALUE_CAP: u64 = 16 * 1024 * 1024;
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut rows = 0_u64;
        let mut bytes = 0_u64;
        let mut node_rows = 0_u64;
        let mut node_bytes = 0_u64;
        for (table, columns) in [
            ("canonical_nodes", &["body", "kind", "logical_id", "source_id"][..]),
            ("canonical_edges", &["body", "kind", "logical_id", "source_id"][..]),
            ("search_index_content", &["c0", "c1"][..]),
            ("search_index_edges_content", &["c0", "c1"][..]),
            ("search_index_v2_content", &["c0", "c1", "c2"][..]),
            ("property_search_index_content", &["c0", "c1"][..]),
        ] {
            let lengths = columns
                .iter()
                .map(|col| format!("COALESCE(octet_length({col}),0)"))
                .collect::<Vec<_>>()
                .join(",");
            let mut statement = c
                .prepare(&format!("SELECT {lengths} FROM {table} LIMIT 100000"))
                .map_err(|_| "storage")?;
            let mut records = statement.query([]).map_err(|_| "storage")?;
            while let Some(row) = records.next().map_err(|_| "budget_exhausted")? {
                rows = rows.checked_add(1).ok_or("budget_exhausted")?;
                if rows >= ROW_CAP {
                    return Err("budget_exhausted");
                }
                let mut length = 512_u64;
                for index in 0..columns.len() {
                    let n: u64 = row.get(index).map_err(|_| "storage")?;
                    if n > VALUE_CAP {
                        return Err("budget_exhausted");
                    }
                    length = length.checked_add(n).ok_or("budget_exhausted")?;
                }
                bytes = bytes.checked_add(length).ok_or("budget_exhausted")?;
                if bytes > BYTE_CAP {
                    return Err("budget_exhausted");
                }
                if table == "canonical_nodes" {
                    node_rows += 1;
                    node_bytes += length;
                }
            }
        }
        if query.route == Route::Property
            && (query.view.include_inactive || query.view.include_superseded)
        {
            let mut count = 0_u64;
            let mut names = 0_u64;
            let mut statement=c.prepare("SELECT octet_length(name) FROM _fathomdb_projection_registry WHERE fts_tokenizer IS NOT NULL LIMIT 100000").map_err(|_|"storage")?;
            let mut declarations = statement.query([]).map_err(|_| "storage")?;
            while let Some(row) = declarations.next().map_err(|_| "budget_exhausted")? {
                count += 1;
                if count >= ROW_CAP {
                    return Err("budget_exhausted");
                }
                names = names
                    .checked_add(row.get::<_, u64>(0).map_err(|_| "storage")?)
                    .ok_or("budget_exhausted")?;
            }
            rows = rows
                .checked_add(node_rows.checked_mul(count).ok_or("budget_exhausted")?)
                .ok_or("budget_exhausted")?;
            bytes = bytes
                .checked_add(node_bytes.checked_mul(count).ok_or("budget_exhausted")?)
                .and_then(|b| node_rows.checked_mul(names).and_then(|n| b.checked_add(n)))
                .ok_or("budget_exhausted")?;
            if rows >= ROW_CAP || bytes > BYTE_CAP {
                return Err("budget_exhausted");
            }
        }
        Ok(
            json!({"context":Self::finished(context),"scope":"before_native_call; separate_read","strategy":"conservative_stored_text_and_relaxed_property_scratch","charged_rows":rows,"charged_bytes":bytes.to_string(),"row_cap":ROW_CAP,"byte_cap":BYTE_CAP,"per_value_byte_cap":VALUE_CAP,"whole_payload_preload":false}),
        )
    }
}

impl Reader {
    fn lexical_metadata(
        c: &crate::session::ReadConnection,
        query: &Query,
    ) -> Result<Value, &'static str> {
        let definitions = match query.route {
            Route::Body => {
                vec![("search_index", vec!["body"]), ("search_index_edges", vec!["body"])]
            }
            Route::Property => vec![("property_search_index", vec!["attr_value"])],
            Route::Bm25f => vec![("search_index_v2", vec!["kind", "body", "status"])],
        };
        let mut indexes = vec![];
        for (table, fields) in definitions {
            let ddl: Option<String> = c
                .query_row(
                    "SELECT substr(sql,1,8192) FROM sqlite_schema WHERE name=?1",
                    [table],
                    |r| r.get(0),
                )
                .map_err(|_| "storage")?;
            indexes.push(json!({"index":table,"fields":fields,"effective_ddl":ddl,"effective_tokenizer":if ddl.as_deref().is_some_and(|v|v.contains("porter unicode61 remove_diacritics 2")){"porter unicode61 remove_diacritics 2"}else{"unknown; inspect actual DDL"}}));
        }
        let declaration = if let Some(name) = query.property.as_deref() {
            c.query_row("SELECT CASE WHEN octet_length(fts_tokenizer)<=4096 THEN fts_tokenizer END,CASE WHEN octet_length(source)<=8192 THEN source END,octet_length(source) FROM _fathomdb_projection_registry WHERE name=?1",[name],|r|Ok(json!({"name":name,"configured_tokenizer":r.get::<_,Option<String>>(0)?,"source_path_json":r.get::<_,Option<String>>(1)?,"source_path_bytes":r.get::<_,Option<i64>>(2)?.map(|n|n.to_string())}))).optional().map_err(|_|"storage")?
        } else {
            None
        };
        Ok(
            json!({"scope":"later_index_metadata_read; not native_query_snapshot","indexes":indexes,"declaration":declaration,"query_interpretation":if query.route==Route::Bm25f{"native_tokenizer_OR_terms"}else{"native_plain_text_compiler"},"match_evidence":if query.route==Route::Body{"actual_engine_snippet_sidecar"}else{"no_native_span_output"}}),
        )
    }
}

impl Reader {
    pub(super) fn lexical_bounded(mut response: Value) -> Result<Value, &'static str> {
        loop {
            if serde_json::to_vec(&response).map_err(|_| "storage")?.len() <= RESPONSE {
                return Ok(response);
            }
            let items = response["items"].as_array_mut().ok_or("budget_exhausted")?;
            if items.pop().is_none() {
                return Err("budget_exhausted");
            }
            let returned = items.len();
            response["status"] = json!("partial");
            response["scope"]["partial"] = json!(true);
            response["scope"]["returned"] = json!(returned);
            response["scope"]["reason"] = json!("serialized_response_byte_cap");
        }
    }
}
