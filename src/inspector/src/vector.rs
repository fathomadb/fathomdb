use super::*;

fn components(bytes: &[u8], dimension: usize) -> Result<Vec<f32>, &'static str> {
    if dimension == 0 || dimension > CHUNK / 4 || bytes.len() != dimension * 4 {
        return Err("unsupported_value_encoding");
    }
    bytes
        .chunks_exact(4)
        .map(|b| {
            let value = f32::from_le_bytes(b.try_into().unwrap());
            if value.is_finite() {
                Ok(value)
            } else {
                Err("unsupported_value_encoding")
            }
        })
        .collect()
}
impl Reader {
    pub(crate) fn vectors(&mut self, handle: &str) -> Result<Value, &'static str> {
        let locator = self
            .attributes
            .get(handle)
            .map(|v| v.origin.clone())
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
        let origin = json!({"handle":origin_handle,"record":record});
        let mut statement=c.prepare("SELECT r.rowid,r.kind,p.profile,p.name,p.revision,p.dimension,s.window_number,s.byte_start,s.byte_end,s.body_length FROM _fathomdb_vector_rows r LEFT JOIN _fathomdb_vector_kinds k ON k.kind=r.kind LEFT JOIN _fathomdb_embedder_profiles p ON p.profile=k.profile LEFT JOIN _fathomdb_vector_spans s ON s.vector_rowid=r.rowid AND s.write_cursor=r.write_cursor WHERE r.write_cursor=?1 ORDER BY r.rowid LIMIT 201").map_err(|_|"storage")?;
        let mut rows = statement.query([locator.cursor]).map_err(|_| "storage")?;
        let mut items = vec![];
        let mut partial = false;
        while let Some(row) = rows.next().map_err(|_| "storage")? {
            if items.len() == 200 {
                partial = true;
                break;
            }
            let rowid: i64 = row.get(0).map_err(|_| "storage")?;
            let dimension: Option<usize> = row.get(5).map_err(|_| "storage")?;
            let blob: Option<Vec<u8>> = c
                .query_row("SELECT embedding FROM vector_default WHERE rowid=?1", [rowid], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(|_| "storage")?;
            let values = match (&blob, dimension) {
                (Some(blob), Some(dimension)) => Some(components(blob, dimension)?),
                _ => None,
            };
            let window: Option<i64> = row.get(6).map_err(|_| "storage")?;
            let start: Option<i64> = row.get(7).map_err(|_| "storage")?;
            let end: Option<i64> = row.get(8).map_err(|_| "storage")?;
            let length: Option<i64> = row.get(9).map_err(|_| "storage")?;
            let window_text = if let Some(end) = end {
                if end < 0 || length.is_none_or(|length| end > length) {
                    return Err("unsupported_value_encoding");
                }
                let content = crate::window::Column::open(
                    &c,
                    locator.class.table(),
                    locator.class.body(),
                    locator.rowid,
                )?;
                let bytes = content.read(0, (end as usize).min(4096))?;
                let (text, n) = utf8_prefix(bytes.as_deref().unwrap_or_default(), 4096)?;
                json!({"text":text,"text_bytes":n,"text_complete":n==end as usize})
            } else {
                json!({"text":null,"text_bytes":0,"text_complete":false})
            };
            let item = json!({"vector_rowid":rowid.to_string(),"source_cursor":locator.cursor.to_string(),"kind":row.get::<_,String>(1).map_err(|_|"storage")?,"profile":row.get::<_,Option<String>>(2).map_err(|_|"storage")?,"producer":{"name":row.get::<_,Option<String>>(3).map_err(|_|"storage")?,"revision":row.get::<_,Option<String>>(4).map_err(|_|"storage")?,"dimension":dimension,"source":"stored_embedder_profile"},"representation":"stored_uncentered_f32","values":values,"window":{"number":window,"byte_start":start.map(|v|v.to_string()),"byte_end":end.map(|v|v.to_string()),"body_length":length.map(|v|v.to_string()),"coverage":if end.is_some(){"recorded"}else{"unknown"},"text":window_text["text"],"text_bytes":window_text["text_bytes"],"text_complete":window_text["text_complete"]},"materialization":if blob.is_some(){"present"}else{"missing"},"origin":{"state":"exact","targets":[origin]},"retrieval_eligibility":"not_evaluated"});
            items.push(item);
        }
        Self::lexical_bounded(
            json!({"status":if partial{"partial"}else{"complete"},"items":items,"context":Self::finished(context),"scope":{"origin":origin,"materialization":if items.is_empty(){"not_observed"}else{"observed"},"retrieval_eligibility":"not_evaluated","representation":"stored_uncentered_f32","policy":self.session.policy,"item_cap":200,"runtime":{"dense_query":if self.session.cached_cpu{"available"}else{"unavailable"},"device":if self.session.cached_cpu{Some("cpu")}else{None},"reason":if self.session.cached_cpu{"production_default_sealed_cache_cpu"}else{"no_qualified_cached_runtime"},"scope":"current_owned_engine_session"},"response_byte_cap":RESPONSE}}),
        )
    }
}

impl Reader {
    pub(crate) fn serving(&mut self, query: &str, limit: usize) -> Result<Value, &'static str> {
        if query.trim().is_empty() || query.len() > 4096 || !(1..=100).contains(&limit) {
            return Err("invalid_range");
        }
        if !self.session.cached_cpu {
            return Err("model_unavailable");
        }
        let admission = self.lexical_admission(&super::lexical::Query {
            route: super::lexical::Route::Body,
            query: query.into(),
            property: None,
            filter: Default::default(),
            view: Default::default(),
            limit,
        })?;
        let request = fathomdb_engine::ReadContextV1::new(
            fathomdb_engine::ReadView::default(),
            fathomdb_engine::SearchFilter::default(),
        )
        .map_err(|_| "storage")?;
        let frozen = self.session.engine.freeze_read_context(&request).map_err(|_| "storage")?;
        let started = crate::session::epoch_ms();
        let result = self
            .session
            .engine
            .search_frozen(query, &frozen, 0, false, 0.3, 0, true, limit)
            .map_err(|_| "native_query_refused")?;
        let ended = crate::session::epoch_ms();
        let explanation = result.explanation.as_ref().ok_or("native_evidence_unavailable")?;
        let trace = &explanation.trace;
        let query_context = json!({"route":"engine_frozen_hybrid","read_started_epoch_ms":started.to_string(),"read_ended_epoch_ms":ended.to_string(),"snapshot":"native_frozen_search","projection_cursor":result.projection_cursor.to_string(),"frozen_token":frozen.token,"effective_valid_at":frozen.effective_valid_at.to_string(),"correlation_id":explanation.correlation_id,"vector_hits":trace.vector_hits,"text_hits":trace.text_hits,"graph_hits":trace.graph_hits,"embedder_id":trace.embedder_id,"ce_active":trace.ce_active,"alpha":trace.alpha,"pool_n":trace.pool_n,"rerank_depth":trace.rerank_depth,"graph_requested":trace.use_graph_arm,"stop_words_sha256":trace.stop_words_sha256,"runtime":"production_default_sealed_cache_cpu","degradations":result.degradations.iter().map(|v|json!({"arm":format!("{:?}",v.arm),"reason":v.reason.as_str()})).collect::<Vec<_>>()});
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let mut remaining = 100000;
        let mut items = vec![];
        let mut withheld = 0;
        for (rank, (hit, evidence)) in result.results.iter().zip(&explanation.per_hit).enumerate() {
            let class = match hit.branch {
                fathomdb_engine::SoftFallbackBranch::TextEdge => Class::Edge,
                _ => Class::Node,
            };
            let cursor =
                i64::try_from(hit.write_cursor).map_err(|_| "unsupported_value_encoding")?;
            let origins = match self.lexical_origins(&c, class, cursor, &mut remaining) {
                Ok(v) if !v.is_empty() => v,
                Ok(_) | Err("refused" | "unavailable" | "reference_expired") => {
                    withheld += 1;
                    continue;
                }
                Err(e) => return Err(e),
            };
            if !hit.score.is_finite()
                || !evidence.fused_score.is_finite()
                || !evidence.blended.is_finite()
            {
                return Err("unsupported_value_encoding");
            }
            let (text, n) = utf8_prefix(hit.body.as_bytes(), 4096)?;
            items.push(json!({"rank":(rank+1).to_string(),"score":hit.score.to_string(),"class":class,"native_id":{"space":hit.id.space.as_str(),"value":if hit.id.value.len()<=4096{Some(&hit.id.value)}else{None}},"write_cursor":hit.write_cursor.to_string(),"representative_branch":format!("{:?}",hit.branch),"payload":{"text":text,"total_bytes":hit.body.len().to_string(),"complete":n==hit.body.len(),"scope":"native_frozen_search"},"evidence":{"vector_rank":evidence.vector_rank,"text_rank":evidence.text_rank,"graph_rank":evidence.graph_rank,"rank_base":0,"fused_score":evidence.fused_score.to_string(),"blended_score":evidence.blended.to_string(),"ce_score":evidence.ce_score,"structural":{"inclusion_state":format!("{:?}",evidence.structural.inclusion_state),"projection_origin":format!("{:?}",evidence.structural.projection_origin),"dependency_state":format!("{:?}",evidence.structural.dependency_state),"lifecycle_state":format!("{:?}",evidence.structural.lifecycle_state),"degradation_codes":evidence.structural.degradation_codes.iter().map(|v|format!("{v:?}")).collect::<Vec<_>>() }},"origin":{"state":if origins.len()==1{"exact"}else{"choices"},"targets":origins,"scope":"separate_later_policy_exact_cursor_read"}}));
        }
        Self::lexical_bounded(
            json!({"status":"complete","items":items,"request":{"query":query,"limit":limit,"view":"engine_default_eligible","filter":null},"admission":admission,"query_context":query_context,"context":Self::finished(context),"scope":{"score_family":"engine_rrf","score_direction":"higher_better","dense_stage_distances":"not_exposed","binary_shortlist":"mean_centered_sign_when_pinned; not_stored_L2","native_returned":result.results.len(),"returned":items.len(),"withheld":withheld,"rank_gaps":"original_native_ranks_preserved","policy":self.session.policy,"response_byte_cap":RESPONSE}}),
        )
    }
}

struct Neighbor {
    distance: f64,
    rowid: i64,
    cursor: i64,
    origins: Vec<Locator>,
}
impl PartialEq for Neighbor {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}
impl Eq for Neighbor {}
impl PartialOrd for Neighbor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Neighbor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance.total_cmp(&other.distance).then(self.rowid.cmp(&other.rowid))
    }
}
fn blob(c: &rusqlite::Connection, rowid: i64) -> Result<Option<(Vec<u8>, String)>, &'static str> {
    c.query_row("SELECT CASE WHEN octet_length(embedding)<=262144 THEN embedding END,source_type FROM vector_default WHERE rowid=?1",[rowid],|r|Ok((r.get::<_,Option<Vec<u8>>>(0)?,r.get::<_,String>(1)?))).optional().map_err(|_|"storage")?.map(|(v,s)|v.map(|v|(v,s)).ok_or("unsupported_value_encoding")).transpose()
}
impl Reader {
    pub(crate) fn neighborhood(
        &mut self,
        handle: &str,
        rowid: i64,
        after: i64,
        limit: usize,
    ) -> Result<Value, &'static str> {
        if rowid <= 0 || after < 0 || !(1..=100).contains(&limit) {
            return Err("invalid_range");
        }
        let locator = self
            .attributes
            .get(handle)
            .map(|v| v.origin.clone())
            .or_else(|| self.handles.get(handle).cloned())
            .ok_or("reference_expired")?;
        if !locator.class.canonical() {
            return Err("unsupported_constraint");
        }
        let c = self.session.connection()?;
        let context = self.context(&c)?;
        let (seed_record, classification) = Self::summary(&c, &locator)?;
        if !self.session.policy.permits(classification) {
            return Err("refused");
        }
        let profile:Option<(String,usize)>=c.query_row("SELECT CASE WHEN octet_length(p.profile)<=4096 THEN p.profile END,p.dimension FROM _fathomdb_vector_rows r JOIN _fathomdb_vector_kinds k ON k.kind=r.kind JOIN _fathomdb_embedder_profiles p ON p.profile=k.profile WHERE r.rowid=?1 AND r.write_cursor=?2",params![rowid,locator.cursor],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|"storage")?;
        let (profile, dimension) = profile.ok_or("reference_expired")?;
        let (seed, source_type) = blob(&c, rowid)?.ok_or("unavailable")?;
        components(&seed, dimension)?;
        let mut statement=c.prepare("SELECT r.rowid,r.write_cursor,k.profile FROM _fathomdb_vector_rows r LEFT JOIN _fathomdb_vector_kinds k ON k.kind=r.kind WHERE r.rowid>?1 ORDER BY r.rowid LIMIT 100000").map_err(|_|"storage")?;
        let mut rows = statement.query([after]).map_err(|_| "storage")?;
        let started = std::time::Instant::now();
        let mut heap = std::collections::BinaryHeap::<Neighbor>::new();
        let (
            mut examined,
            mut visible,
            mut eligible,
            mut source_rows,
            mut representations,
            mut work,
        ) = (0, 0, 0, 0, 0, 0);
        let mut partial = false;
        let mut last = after;
        while let Some(row) = rows.next().map_err(|_| "budget_exhausted")? {
            examined += 1;
            work += 1;
            if work >= 100000 || started.elapsed() >= WORK_TIME {
                partial = true;
                break;
            }
            let candidate: i64 = row.get(0).map_err(|_| "storage")?;
            let cursor: i64 = row.get(1).map_err(|_| "storage")?;
            let candidate_profile: Option<String> = row.get(2).map_err(|_| "storage")?;
            if candidate_profile.as_deref() != Some(&profile) {
                last = candidate;
                continue;
            }
            if work >= 99999 {
                partial = true;
                break;
            }
            let Some((vector, source_type)) = blob(&c, candidate)? else {
                last = candidate;
                continue;
            };
            representations += 1;
            work += 1;
            let class = if source_type == "edge_fact" { Class::Edge } else { Class::Node };
            let mut origins = vec![];
            let mut owners = c
                .prepare(&format!(
                    "SELECT rowid FROM {} WHERE write_cursor=?1 LIMIT 3",
                    class.table()
                ))
                .map_err(|_| "storage")?;
            let mut owners = owners.query([cursor]).map_err(|_| "storage")?;
            let mut permitted = true;
            while let Some(owner) = owners.next().map_err(|_| "storage")? {
                if work >= 100000 {
                    partial = true;
                    permitted = false;
                    break;
                }
                work += 1;
                source_rows += 1;
                let owner_row: i64 = owner.get(0).map_err(|_| "storage")?;
                let revision_rowid=c.query_row("SELECT rowid FROM _fathomdb_artifact_revisions WHERE artifact_class=?1 AND write_cursor=?2",params![class.label(),cursor],|r|r.get(0)).optional().map_err(|_|"storage")?;
                let origin = Locator { class, rowid: owner_row, cursor, revision_rowid };
                let (_, classification) =
                    Self::summary_fields(&c, &origin, &["kind", "logical_id", "source_id"])?;
                if !self.session.policy.permits(classification) {
                    permitted = false;
                    break;
                }
                origins.push(origin);
            }
            if partial {
                break;
            }
            last = candidate;
            if !permitted || origins.is_empty() {
                continue;
            }
            visible += 1;
            components(&vector, dimension)?;
            let distance: f64 = c
                .query_row("SELECT vec_distance_l2(?1,?2)", params![seed, vector], |r| r.get(0))
                .map_err(|_| "unsupported_value_encoding")?;
            if !distance.is_finite() || distance < 0.0 {
                return Err("unsupported_value_encoding");
            }
            eligible += 1;
            let item = Neighbor { distance, rowid: candidate, cursor, origins };
            if heap.len() < limit {
                heap.push(item);
            } else if heap.peek().is_some_and(|worst| item < *worst) {
                heap.pop();
                heap.push(item);
            }
            if examined == 100000 {
                partial = true;
                break;
            }
        }
        let mut items = vec![];
        for (rank, item) in heap.into_sorted_vec().into_iter().enumerate() {
            let mut origins = vec![];
            for locator in item.origins {
                let (record, classification) = Self::summary(&c, &locator)?;
                if !self.session.policy.permits(classification) {
                    return Err("refused");
                }
                origins.push(json!({"handle":self.mint(locator),"record":record}));
            }
            items.push(json!({"rank":(rank+1).to_string(),"distance":item.distance.to_string(),"vector_rowid":item.rowid.to_string(),"source_cursor":item.cursor.to_string(),"profile":profile,"origin":{"state":if origins.len()==1{"exact"}else{"choices"},"targets":origins}}));
        }
        Self::lexical_bounded(
            json!({"status":if partial{"partial"}else{"complete"},"items":items,"request":{"handle":handle,"vector_rowid":rowid.to_string(),"after":after.to_string(),"limit":limit},"seed":{"vector_rowid":rowid.to_string(),"source_cursor":locator.cursor.to_string(),"profile":profile,"source_type":source_type,"dimension":dimension,"origin":{"handle":handle,"record":seed_record}},"context":Self::finished(context),"scope":{"metric":"uncentered_f32_euclidean_l2","direction":"lower_nearer","numeric":"sqlite_vec_f32_accumulation_sqrt","serving_eligibility":"not_evaluated","population":"compatible_profile_policy_visible_stored_vectors_in_compared_rowid_window","source_types":"all","kinds":"all","examined":examined,"source_rows_examined":source_rows,"representation_reads":representations,"work":work,"work_cap":100000,"visible":visible,"eligible_compared":eligible,"returned":items.len(),"rowid_after":after.to_string(),"rowid_end":last.to_string(),"continuation":if partial{Some(last.to_string())}else{None},"partial":partial,"nearest_scope":"compared_population_only","policy":self.session.policy,"response_byte_cap":RESPONSE,"deadline_millis":WORK_TIME.as_millis()}}),
        )
    }
}
