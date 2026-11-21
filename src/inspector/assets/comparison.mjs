// Adapters reference admitted envelopes; they never query or copy result payloads.
const canonical = record => [record.class, record.write_cursor, record.revision_id ?? null];

/** Return bounded typed entries referencing one admitted observation, with origins separate. */
export function entries(data, request = {}) {
  if (!data || data.evicted) return [];
  const base = [data.context?.database, data.context?.attachment];
  const graph = request.op === 'graph' || data.scope?.route === 'stored_adjacency';
  const frozen = request.op === 'graph_serving' || data.scope?.route === 'actual_serving_graph_evidence';
  const source = graph ? [...(data.nodes ?? []), ...(data.edges ?? [])] : data.items ?? [];
  if (source.length > 200) throw Error('comparison_item_limit');
  return source.map((item, ordinal) => {
    let identity, origins = item.origin?.targets ?? [];
    if (graph) {
      identity = [...base, 'canonical', ...canonical(item)];
      origins = [{handle:item.handle, record:item}];
    } else if (frozen) {
      identity = [...base, 'frozen_graph_path', ...canonical(item.target), ...canonical(item.terminal_edge)];
      origins = [item.target, item.terminal_edge, ...(item.origin_records ?? [])].map(record => ({handle:record.handle, record}));
    } else if (item.vector_rowid !== undefined) {
      identity = [...base, 'vector_window', item.profile ?? null, item.vector_rowid, item.source_cursor];
    } else if (request.op === 'page' || request.op === 'sample') {
      identity = [...base, item.class === 'node' || item.class === 'edge' ? 'canonical' : 'operational', ...canonical(item), item.collection_name ?? null, item.record_key ?? null, item.entry_id ?? null];
      // Canonical browsing and a stored graph refer to the same exact physical revision.
      if (item.class === 'node' || item.class === 'edge') identity.length -= 3;
      origins = [{handle:item.handle, record:item, metadataOnly:item.payload_status !== 'available'}];
    } else if (item.native_id?.value != null && item.native_id.complete !== false) {
      identity = [...base, 'native_result', item.class, item.native_id.space, item.native_id.value, item.write_cursor];
    } else if (item.representative_branch === 'native_bm25f_substrate') {
      identity = [...base, 'bm25f_substrate', item.class, item.write_cursor];
    } else {
      // Missing physical identity is observation-local, never inferred from equal bodies/origins.
      identity = [...base, 'unresolved_result', data.observation ?? data.context?.operation, ordinal];
    }
    const membershipOnly = graph || frozen || request.op === 'page' || request.op === 'sample';
    return {
      identity, key:JSON.stringify(identity), item, origins,
      label:item.native_id?.value ?? item.logical_id ?? origins[0]?.record?.logical_id ?? `entry ${item.vector_rowid ?? item.write_cursor ?? ordinal}`,
      rank:membershipOnly ? null : item.rank ?? null,
      value:membershipOnly ? null : item.distance ?? item.score ?? null,
      metric:membershipOnly ? 'not applicable (membership only)' : data.scope?.metric ?? data.scope?.score_family ?? 'unknown',
      direction:membershipOnly ? 'not applicable' : data.scope?.direction ?? data.scope?.score_direction ?? 'unknown',
    };
  });
}

/** Compare actual entry multisets, returning at most 50 rows and a bounded union count. */
export function comparisonPage(left, right, {mode='union',start=0,limit=50} = {}) {
  if (left.length > 200 || right.length > 200) throw Error('comparison_item_limit');
  if (!['union','intersection'].includes(mode)) throw Error('invalid_comparison_mode');
  start = Math.max(0, Math.floor(start));
  limit = Math.max(1, Math.min(50, Math.floor(limit)));
  const indices = new Map(), used = new Set(), rows=[];
  right.forEach((entry,index) => {
    const group=indices.get(entry.key) ?? [];
    group.push(index);indices.set(entry.key,group);
  });
  let total=0;
  const emit=(A,B)=>{if(total>=start && rows.length<limit)rows.push({A,B});total++;};
  for (const A of left) {
    const match=indices.get(A.key)?.shift();
    if (match !== undefined) {used.add(match);emit(A,right[match]);}
    else if (mode === 'union') emit(A,null);
  }
  if (mode === 'union') right.forEach((B,index)=>{if(!used.has(index))emit(null,B);});
  return {rows,total,start,limit};
}

/** Identify a single verified exact origin independently of native result identity. */
export function canonicalOriginKey(entry) {
  if (!entry || entry.origins?.length !== 1) return null;
  const origin=entry.origins[0],record=origin.record;
  if(origin.metadataOnly || !origin.handle || !record || !['node','edge'].includes(record.class))return null;
  if(!entry.identity?.[0] || !entry.identity?.[1] || typeof record.write_cursor!=='string' || !/^[1-9]\d*$/.test(record.write_cursor) || typeof record.revision_id!=='string' || !record.revision_id)return null;
  return JSON.stringify([...entry.identity.slice(0,2),record.class,record.write_cursor,record.revision_id]);
}

/** Return bounded shared exact-origin groups without merging or reranking native rows. */
export function canonicalCorrespondence(left,right) {
  if(left.length>200 || right.length>200)throw Error('comparison_item_limit');
  const groups=new Map();
  for(const [side,values] of [['A',left],['B',right]])for(const entry of values){
    const key=canonicalOriginKey(entry);if(key===null)continue;
    const group=groups.get(key)??{A:[],B:[]};group[side].push(entry);groups.set(key,group);
  }
  for(const [key,group] of groups)if(!group.A.length||!group.B.length)groups.delete(key);
  return groups;
}
