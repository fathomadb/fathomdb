import { propertyChoices, epochUTC } from "/presentation.mjs";
import { entries, comparisonPage, canonicalCorrespondence } from "/comparison.mjs";
const BUILD = "__INSPECTOR_BUILD__";
const root = document.getElementById("app"),
  notice = document.getElementById("notice");
let visibilityKey = null,
  cacheVersion = 0,
  polling = false;
let pair,
  initialPath = "",
  view,
  history = [],
  epoch = 0,
  serial = 0,
  navigation = 0,
  activeScope = null,
  discoveredScopes = [];
function updateDiscoveredScopes(catalog) {
  if (Array.isArray(catalog?.scopes)) {
    discoveredScopes = catalog.scopes.map((entry) => ({ ...entry }));
  }
}
function isMultiScope() {
  return discoveredScopes.length > 1 || discoveredScopes.some((entry) => entry.scope !== "");
}
function scopeLabel(value) {
  if (value === null || value === undefined) return "All visible scopes";
  return value === "" ? '"" (default scope)' : String(value);
}
function decodeScopeOption(raw) {
  if (raw === "__all__" || raw === "__default__") return null;
  return raw.startsWith("scope:") ? raw.slice(6) : raw;
}
function buildScopeSelect(ariaLabel, selected, includeDefaultUnscoped = false) {
  const select = el("select", null, { "aria-label": ariaLabel });
  if (includeDefaultUnscoped) {
    select.append(el("option", 'Default scope (unscoped "")', { value: "__default__" }));
  } else {
    select.append(el("option", "All visible scopes", { value: "__all__" }));
  }
  const seen = new Set();
  for (const entry of discoveredScopes) {
    seen.add(entry.scope);
    const counts =
      entry.total !== undefined
        ? ` (${entry.total} total: ${entry.node} nodes, ${entry.edge} edges, ${entry.current} current, ${entry.log} log)`
        : "";
    select.append(
      el("option", `${scopeLabel(entry.scope)}${counts}`, { value: `scope:${entry.scope}` }),
    );
  }
  if (selected !== null && selected !== undefined && !seen.has(selected)) {
    select.append(el("option", scopeLabel(selected), { value: `scope:${selected}` }));
  }
  select.value =
    selected === null || selected === undefined
      ? includeDefaultUnscoped
        ? "__default__"
        : "__all__"
      : `scope:${selected}`;
  return select;
}
const observations = new Map();
const pins = {A:{id:null,version:0}, B:{id:null,version:0}};
let comparisonBytes=0, feedbackSerial=0;
const cacheCap=20*1024*1024;
const bytes=value=>new TextEncoder().encode(JSON.stringify(value)).length;
function scrubDOM(node) {
  if(!node)return;
  for(const child of [...node.children])scrubDOM(child);
  node.replaceChildren();
}
function clearComparison() {
  comparisonBytes=0;
  for(const saved of [view,...history])if(saved?.kind==="comparison") {
    saved.returnFocusText=saved.returnFocus?.textContent??saved.returnFocusText;
    saved.returnFocus=null;scrubDOM(saved.workspace);saved.workspace=null;
  }
}
function cacheBytes() {return comparisonBytes+Object.values(pins).reduce((n,p)=>n+(p.descriptor?bytes(p.descriptor)*2+1024:0),0)+[...observations.values()].reduce((n,item)=>n+item.bytes+item.clear.size*1024,0);}
function disposeView(saved) {
  if(!saved)return;
  scrubDOM(saved.workspace);saved.workspace=null;saved.returnFocus=null;
  for(const item of observations.values())item.clear.delete(saved);
}

function el(tag, text, attrs = {}) {
  const n = document.createElement(tag);
  if (text !== undefined && text !== null) n.textContent = String(text);
  for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, String(v));
  return n;
}
function button(label, action, attrs = {}) {
  const b = el("button", label, attrs);
  b.addEventListener("click", () => run(action));
  return b;
}
function recordDisplay(parent, preview) {
  if(!preview)return;
  if(preview.state==="available"){
    parent.append(el("p",`${preview.text}${preview.complete?"":"…"}`,{class:"record-display"}));return;
  }
  const reasons={no_display_field:"no stored title/name in the inspected body",oversize_not_inspected:"body exceeds bounded preview limit",preview_budget_exhausted:"shared preview budget exhausted",null:"stored body is null"};
  parent.append(el("p",`Stored label unavailable: ${reasons[preview.state]??"unavailable in this observation"}; use exact identity.`,{class:"record-display-fallback muted"}));
}
function attachmentStatus() {
  document.getElementById("session-status").textContent = pair?.attachment ? "Attached database · owned Engine session." : "Local session authenticated. No database attached.";
}
function say(text) {
  notice.textContent = text;
}
function clear() {
  root.replaceChildren();
  root.setAttribute("aria-busy", "false");
}
function strip(value) {
  if (value && typeof value === "object") {
    for (const key of Object.keys(value)) {
      strip(value[key]);
      delete value[key];
    }
  }
}
function evict(value, item) {
  const context = value.context ? structuredClone(value.context) : null;
  for (const saved of [view, ...history])
    if (saved && (saved.data === value || saved.source === value)) {
      saved.context = context;
      if(saved.source===value&&saved.data!==value&&!saved.data?.evicted)saved.returnFocusText=saved.returnFocus?.textContent??saved.returnFocusText;
      else delete saved.returnFocusText;
      scrubDOM(saved.workspace);saved.workspace=null;saved.returnFocus=null;
      if(view===saved)queueMicrotask(()=>{if(view===saved)render();});
    }
  for(const clear of item?.clear?.values()??[])clear();
  item?.clear?.clear();
  for(const slot of Object.values(pins))if(slot.id===value.observation){slot.id=null;slot.descriptor=null;slot.version++;slot.state="evicted or invalidated";clearComparison();}
  strip(value);
  value.evicted = true;
}
function invalidate(message) {
  cacheVersion++;
  for (const item of observations.values()) evict(item.data, item);
  observations.clear();
  if (view) render();
  say(message);
}
function acceptContext(context) {
  if (!context) return;
  const key = JSON.stringify([
    context.attachment,
    context.visibility_generation,
    context.visibility_nonce,
    context.projection_generation,
    context.dependency_generation,
  ]);
  if (visibilityKey !== null && visibilityKey !== key)
    invalidate("Database observation changed. Cached values cleared; refresh explicitly.");
  visibilityKey = key;
}
function observe(value,request) {
  const id = ++serial;
  Object.defineProperty(value, "observation", { value: id });
  observations.set(id, {
    data: value,
    bytes: bytes(value)*2+bytes(request)*2+1024,
    request: structuredClone(request),
    clear: new Map(),
  });
  while (
    [...observations.values()].reduce((n,x)=>n+(x.data.nodes?.length??0)+(x.data.edges?.length??0)+(x.data.scope?.route==="actual_serving_graph_evidence"?(x.data.items?.length??0)*4:0),0)>500 ||
    observations.size > 10 ||
    cacheBytes() > cacheCap
  ) {
    const [old, item] = observations.entries().next().value;
    observations.delete(old);
    evict(item.data, item);
    say(
      "An earlier observation was evicted. Its locator is retained; refresh explicitly to read again.",
    );
  }
  return value;
}
function bind(value, clear) {
  const item = observations.get(value.observation);
  if (item) item.clear.set(view,clear);
}
function forget() {
  visibilityKey = null;
  clearComparison();
  for(const item of observations.values())evict(item.data,item);
  observations.clear();
  for(const slot of Object.values(pins)){slot.id=null;slot.descriptor=null;slot.version++;delete slot.state;}
  for(const saved of [view,...history])disposeView(saved);
  history = [];
  epoch++;
  view = null;
  activeScope = null;
  discoveredScopes = [];
  attachmentStatus();
}
async function api(request, retain = true, accept = () => true) {
  const at = epoch,
    version = cacheVersion;
  root.setAttribute("aria-busy", "true");
  let response, data;
  try {
  response = await fetch("/api", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ...pair, request }),
  });
  data = await response.json();
  } catch (error) {
    if (at !== epoch || !accept()) throw Object.assign(error, {silent:true});
    throw error;
  }
  if (at !== epoch || !accept()) throw Object.assign(Error("superseded"), { silent: true });
  root.setAttribute("aria-busy", "false");
  if (version !== cacheVersion) throw Error("This response was invalidated; refresh explicitly.");
  if ("attachment" in data) { pair.attachment = data.attachment; attachmentStatus(); }
  if (request.op === "attach" && data.status === "complete") say("");
  if (!response.ok || !["complete", "partial"].includes(data.status)) {
    if (
      data.status === "cancelled" ||
      ("attachment" in data && data.attachment === null && request.op !== "detach")
    ) {
      forget();
      selector();
    } else if (["expired", "refused", "unavailable"].includes(data.status) && pair.attachment) {
      invalidate("This observation is no longer readable. Refresh explicitly.");
    }
    throw Object.assign(Error(data.error ?? data.status), { details: data });
  }
  acceptContext(data.context);
  return retain ? observe(data,request) : data;
}
async function validate(current, accept = () => true) {
  const at = epoch;
  try {
    const handles = new Set(), metadataHandles = new Set();
    if (current?.kind === "detail") {
      handles.add(current.handle);
      if (current.data?.origin_reference) handles.add(current.data.origin_reference.handle);
      const data = current.associations;
      if (data && !data.evicted) {
        for (const link of data.links) for (const target of link.targets) handles.add(target.handle);
        for (const attribute of data.attributes) if (attribute.handle) handles.add(attribute.handle);
      }
    } else if (current?.kind === "graph_evidence") {
      const item=observations.get(current.data?.observation);
      if(item)collectComparisonHandles(item,handles,metadataHandles);
    } else if (current?.kind === "graph" || current?.kind === "investigation") {
      handles.add(current.handle);
      for(const data of [current.data,current.native]) {
        if(!data||data.evicted)continue;
        for(const record of [...(data.nodes??[]),...(data.edges??[]),...(data.revisions??[])])handles.add(record.handle);
        for(const link of data.links??[])if(link.target)handles.add(link.target.handle);
        for(const item of data.items??[])for(const record of [item.target,item.terminal_edge,...(item.origin_records??[])])if(record)handles.add(record.handle);
      }
    } else if (current?.kind === "vector") {
      if(current.handle)handles.add(current.handle);
      for(const data of [current.source,current.data]){
        if(data?.scope?.origin?.handle)handles.add(data.scope.origin.handle);
        if(data?.seed?.origin?.handle)handles.add(data.seed.origin.handle);
        for(const item of data?.items??[])for(const origin of item.origin?.targets??[])handles.add(origin.handle);
      }
    } else if (current?.kind === "lexical") {
      if (current.handle) handles.add(current.handle);
      for (const item of current.data?.items ?? [])
        for (const origin of item.origin?.targets ?? []) handles.add(origin.handle);
      if (current.source?.scope?.origin?.handle) handles.add(current.source.scope.origin.handle);
    } else if (current?.kind === "page" && current.data?.items) {
      for (const item of current.data.items)
        (item.payload_status === "available" ? handles : metadataHandles).add(item.handle);
    }
    for(const slot of Object.values(pins)) {
      const item=observations.get(slot.id);
      if(!item)continue;
      collectComparisonHandles(item,handles,metadataHandles);
    }
    const all=[...[...handles].map(handle=>[handle,true]),...[...metadataHandles].filter(handle=>!handles.has(handle)).map(handle=>[handle,false])];
    for(let start=0;start<all.length||start===0;start+=201) {
      const batch=all.slice(start,start+201);
      await api({op:"check",handles:batch.filter(([,content])=>content).map(([handle])=>handle),metadata_handles:batch.filter(([,content])=>!content).map(([handle])=>handle)},false,accept);
    }
    return true;
  } catch (error) {
    if (error.silent || at !== epoch || !accept()) return false;
    if (pair?.attachment) {
      invalidate("This observation is no longer readable. Refresh explicitly.");
      return false;
    }
    throw error;
  }
}
async function refreshView() {
  const current = view;
  if (!current) return;
  const token = ++navigation;
  const accept = () => token === navigation && view === current;
  if (current.kind === "detail") {
    current.data = await api({
      op: "detail",
      handle: current.handle,
      field: current.field ?? "body",
    }, true, accept);
    scrubDOM(current.workspace);
    current.workspace = null;
    current.associations = null;
    render();
  } else if (current.kind === "graph") {
    await updateGraph(current,current.request);
  } else if (current.kind === "investigation") {
    current.data=await api({op:"investigate",handle:current.handle},true,accept);current.workspace=null;render();
  } else if (current.kind === "vector") {
    current.source=await api(current.handle?{op:"vectors",handle:current.handle}:{op:"catalog"},true,accept);
    if(!current.handle)updateDiscoveredScopes(current.source);
    current.data=current.lastRequest?await api(current.lastRequest,true,accept):current.source;
    current.workspace=null;render();
  } else if (current.kind === "lexical") {
    current.source = await api(current.handle
      ? { op: "indexed", handle: current.handle, offset: "0", limit: 4096 }
      : { op: "catalog" }, true, accept);
    current.declarations=current.handle?await api({op:"catalog"},true,accept):current.source;
    updateDiscoveredScopes(current.declarations);
    current.data = current.request.query.trim()
      ? await api({ op: "lexical", ...current.request }, true, accept)
      : current.source;
    current.workspace = null;
    render();
  } else if (current.kind === "page") {
    current.data = await api({
      op: current.sample ? "sample" : "page",
      class: current.className,
      filter: current.filter,
      after: current.after,
      ...(current.sample ? {} : { limit: 50 }),
    }, true, accept);
    render();
  } else await inventory(current.activeScope ?? activeScope);
}
async function run(action) {
  const feedback = ++feedbackSerial;
  try {
    await action();
  } catch (error) {
    if (error.silent || feedback !== feedbackSerial) return;
    const messages = {
      initial_sidecar:
        "Attachment refused: a nonempty WAL or journal exists. Preserve the database and sidecars; investigate and stop their owner. Do not delete sidecars.",
      unsupported_encoding:
        "Attachment refused: this build reads UTF-8 database text only. The existing file was preserved.",
      unsupported_schema:
        "Attachment refused: schema differs from this compiled build. No migration was attempted.",
      missing_lock:
        "Attachment refused: the existing product lock is missing. Preserve the store and investigate its owner.",
      locked: "Attachment refused: another owner holds the database lock.",
      missing: "Select an existing database file.",
      empty: "Attachment refused: the selected file is empty.",
      startup_failed_effects_possible:
        "Engine startup failed after admission; qualified Engine effects may have occurred.",
      refused: "The active launch policy withholds this content.",
      unavailable: "The exact retained content is unavailable.",
      expired: "This reference or attachment expired. Select the database again.",
      busy: "The request queue is full. Retry after current work finishes.",
      cancelled: "Request cancelled; the session has expired.",
      not_json: "This stored value is text, rather than a supported structured value.",
      invalid_as_of:
        "Invalid snapshot parameter: specify either a non-negative integer write cursor or a signed integer commit time in milliseconds, not both.",
      unsupported_field: "The requested field is not supported for this record class.",
    };
    say(
      error.message === "unsupported_schema"
        ? `Attachment refused: observed schema ${error.details?.observed_schema ?? "unknown"}; supported schema ${error.details?.supported_schema ?? "unknown"}. No migration was attempted.`
        : (messages[error.message] ??
            `Operation did not complete (${/^[a-z_]+$/.test(error.message) ? error.message : "error"}).`),
    );
  }
}
function remember(current = view, focus = document.activeElement) {
  if (current) {
    current.returnFocus = focus;
    history.push(current);
    if (history.length > 10) {
      disposeView(history.shift());
      say("Earlier navigation context evicted.");
    }
  }
}
async function cancelRequest() {
  navigation++;
  epoch++;
  const response = await fetch("/api", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ...pair, request: { op: "cancel" } }),
  });
  const data = await response.json();
  pair.attachment = null;
  forget();
  selector();
  say(
    data.status === "cancelled"
      ? "Cancellation acknowledged. The owned session is terminating."
      : "Cancellation failed.",
  );
}
function actions(parent) {
  const bar = el("div", null, { class: "actions" });
  bar.append(
    button(
      "Back",
      async () => {
        const departing=view,previous = history.pop();
        if (!previous) return;
        const token = ++navigation;
        view = previous;
        await validate(previous, () => token === navigation && view === previous);
        if (token !== navigation) return;
        const released=[departing?.data,departing?.source,departing?.native,departing?.associations].map(data=>data?.observation);
        disposeView(departing);
        for(const id of released)releasePinOnly(id);
        render();
        if (previous.returnFocus?.isConnected) previous.returnFocus.focus();
        else if(previous.returnFocusText)[...root.querySelectorAll("button")].find(b=>b.textContent===previous.returnFocusText)?.focus();
      },
      { disabled: history.length === 0 ? "disabled" : undefined },
    ),
  );
  if (history.length === 0) bar.firstChild.disabled = true;
  else bar.firstChild.removeAttribute("disabled");
  bar.append(
    button("Detach database", async () => {
      const token = ++navigation;
      epoch++;
      await api({ op: "detach" }, false, () => token === navigation);
      forget();
      selector();
      say("Detached. Owned database handles and workers released.");
    }),
    button("Cancel request", cancelRequest),
  );
  parent.append(bar);
}
function labelInput(label, value = "") {
  const node = el("label", label),
    input = el("input");
  input.value = value;
  node.append(input);
  return [node, input];
}
function selector() {
  clear();
  const panel = el("section", null, { class: "panel" });
  panel.append(el("h2", "Attach a local database"));
  panel.append(
    el(
      "p",
      "Inspector owns an Engine session. Normal startup, background maintenance and shutdown may change files. Stop every external writer before attaching. Preserve any existing WAL or journal.",
    ),
  );
  const [label, path] = labelInput("Database path", initialPath);
  panel.append(label);
  panel.append(
    el(
      "p",
      `Launch policy: inactive/history ${pair?.policy?.no_show_inactive ? "hidden" : "visible"}; retained erasure content ${pair?.policy?.show_erased ? "permitted if available" : "withheld"}; telemetry payloads ${pair?.policy?.show_payloads ? "permitted" : "withheld"}.`,
      { class: "muted" },
    ),
  );
  panel.append(
    button(
      "Attach database",
      async () => {
        initialPath = path.value;
        const token = ++navigation;
        await api({ op: "attach", path: path.value }, true, () => token === navigation);
        await inventory();
      },
      { class: "primary" },
    ),
  );
  panel.append(button("Cancel request", cancelRequest));
  root.append(panel);
  view = null;
}
async function inventory(scopeOverride = undefined) {
  if (scopeOverride !== undefined) activeScope = scopeOverride;
  const token = ++navigation;
  const request = activeScope !== null ? { op: "catalog", scope: activeScope } : { op: "catalog" };
  const data = await api(request, true, () => token === navigation);
  updateDiscoveredScopes(data);
  view = { kind: "inventory", data, activeScope };
  render();
}
function observationSummary(parent,data,submitted={}) {
  const request=data.request??observations.get(data.observation)?.request??submitted;
  const section=el("section",null,{class:"observation-summary","aria-label":"Observation summary"});section.append(el("h3","Observation summary"));
  const short=value=>{const text=typeof value==="string"?value:JSON.stringify(value??"unknown");return text.length>220?text.slice(0,220)+"… (full value in exact evidence)":text;};
  const add=(name,value)=>section.append(el("p",`${name}: ${short(value)}`));
  const timed=context=>{const raw=context?.read_started_epoch_ms;return raw?`${epochUTC(raw,"milliseconds")??"UTC unavailable"} · exact epoch milliseconds ${raw}`:"unknown (no time supplied)";};
  add("Method",data.query_context?.route??data.scope?.route??request.route??request.op??"unknown");
  if(request.query!==undefined)add("Observed query",request.query);
  if(request.property)add("Property",request.property);
  if(data.seed)add("Seed",data.seed.origin?.record?.logical_id??data.seed.logical_id??data.seed.vector_rowid??data.seed.key??"exact seed in evidence");
  add("Filters",request.filter??"not supplied by this route");add("View",request.view??"not supplied by this route");
  add("Result limit",data.scope?.limit??request.limit??"unknown");
  add("Metric / direction",`${data.scope?.metric??data.scope?.score_family??"not supplied"} / ${data.scope?.direction??data.scope?.score_direction??"not supplied"}`);
  const asOf=data.query_context?.as_of??data.scope?.as_of??data.request?.as_of;
  if(asOf){
    const label=asOf.mode==="write_cursor"
      ?`write_cursor ${asOf.requested_as_of_write_cursor} (resolved cursor ${asOf.resolved_as_of_write_cursor})`
      :asOf.mode==="epoch_ms"
        ?`epoch_ms ${asOf.requested_as_of_epoch_ms} (resolved cursor ${asOf.resolved_as_of_write_cursor})`
        :"current";
    add("Snapshot (as_of)",label);
  }
  if(data.query_context)add("Native retrieval time",timed(data.query_context));
  add(data.query_context?"Later policy/origin read":"Inspection read time",timed(data.context));
  add("Read source",data.context?.route??"unknown");
  add("Outcome",`${data.status??"unknown"}${data.scope?.reason?" · "+data.scope.reason:""}`);
  parent.append(section);return section;
}
function exactEvidence(parent,label,node) {
  const details=el("details",null,{class:"exact-evidence"});details.append(el("summary",label),node);parent.append(details);return details;
}
function boundEvidence(parent) {
  for(const node of parent.querySelectorAll("pre.query-context,pre.indexed-context,pre.chunk-context,pre.vector-context,pre.serving-context,pre.graph-context,pre.investigation-context,pre.native-graph-context"))node.classList.add("raw-evidence");
}
function scope(parent, data) {
  if(!parent.querySelector(".observation-summary"))observationSummary(parent,data);
  parent.append(el("pre",JSON.stringify({status:data.status,scope:data.scope,observation:data.context},null,2),{class:"scope raw-evidence","aria-label":"Exact scope and read context"}));
  boundEvidence(parent);
}
function metadata(parent, record, current = null) {
  const dl = el("dl");
  for (const [key, value] of Object.entries(record)) {
    if (
      key === "handle" || key === "display_preview" ||
      key === "properties" || key === "properties_status" ||
      key.endsWith("_bytes") ||
      key.endsWith("_storage") ||
      key.endsWith("_representation")
    )
      continue;
    const dd = el("dd");
    if (value === null)
      dd.append(el("span", key === "revision_id" ? "No recorded revision evidence" : "SQL NULL"));
    else if (value === "") dd.append(el("span", "Empty string"));
    else dd.append(el("span", typeof value === "object" ? JSON.stringify(value) : String(value)));
    if (record[key + "_bytes"] !== undefined && record[key + "_bytes"] !== null) {
      const shown = new TextEncoder().encode(String(value ?? "")).length,
        total = Number(record[key + "_bytes"]);
      if (shown < total)
        dd.append(
          el("p", `Preview: ${shown} of ${total} stored bytes; open or copy the full field.`, {
            class: "muted",
          }),
        );
    }
    const utc=typeof value==="string"&&(key.endsWith("_epoch_s")||["valid_from","valid_until","t_valid","t_invalid"].includes(key))?epochUTC(value):key.endsWith("_epoch_ms")?epochUTC(value,"milliseconds"):null;
    if(utc)dd.append(el("p",`UTC: ${utc}`,{class:"time-readable"}));
    if (record[key + "_representation"])
      dd.append(el("p", record[key + "_representation"], { class: "muted" }));
    if (
      current &&
      (record[key + "_storage"] !== undefined ||
        (key === "revision_id" && value !== null) ||
        (key === "entry_id" && record.class === "log"))
    ) {
      dd.append(
        button(`Open full ${key}`, () => openField(current, key)),
        button(`Copy full ${key}`, () => copyField(current.handle, key)),
      );
    }
    dl.append(el("dt", key.replaceAll("_", " ")), dd);
  }
  parent.append(dl);
}
async function openDetail(handle, field = "body") {
  const current = view, focus = document.activeElement, token = ++navigation;
  const data = await api({ op: "detail", handle, field }, true, () => token === navigation);
  remember(current, focus);
  view = { kind: "detail", data, handle, field };
  render();
}
async function openField(current, field) {
  await openDetail(current.handle, field);
}

function render() {
  if(view?.kind!=="comparison")clearComparison();
  clear();
  if(view?.kind==="comparison"){renderComparison();return;}
  if (!view?.data || view.data.evicted || (view.source?.evicted && !["lexical","vector"].includes(view.kind))) {
    const panel = el("section", null, { class: "panel" });
    actions(panel);
    panel.append(
      el("p", "Observation evicted or invalidated. Refresh explicitly to read again."),
      button("Refresh this view", refreshView),
    );
    if(view?.queryFeedback)panel.append(el("div",view.queryFeedback,{role:"status","aria-label":"Lexical query feedback"}));
    if (view?.context)
      panel.append(
        el("h3", "Previous observation context"),
        el("pre", JSON.stringify(view.context, null, 2), { class: "previous-context" }),
      );
    root.append(panel);
    return;
  }
  if (view.kind === "inventory") renderInventory();
  else if (view.kind === "page") renderPage();
  else if (view.kind === "detail") renderDetail();
  else if (view.kind === "projection") renderProjection();
  else if (view.kind === "lexical") renderLexical();
  else if (view.kind === "vector") renderVector();
  else if (view.kind === "graph") renderGraph();
  else if (view.kind === "graph_evidence") renderFrozenGraph();
  else if (view.kind === "investigation") renderInvestigation();
  for(const control of root.querySelectorAll("[data-pin-side]"))control.textContent=`${pins[control.dataset.pinSide].id?"Replace":"Pin"} ${control.dataset.pinPrefix}${control.dataset.pinSide}`;
}
function shell() {
  const layout = el("div", null, { class: "layout" }),
    nav = el("nav", null, { "aria-label": "Database structures", class: "panel" }),
    panel = el("section", null, { class: "panel" }),
    scoped = () => (activeScope !== null ? { scope: activeScope } : {});
  nav.append(
    button("Database inventory", () => inventory()),
    button("Canonical nodes", () => page("node", scoped())),
    button("Canonical edges", () => page("edge", scoped())),
    button("Operational current values", () => page("current", scoped())),
    button("Retained mutation entries", () => page("log", scoped())),
  );
  nav.append(el("h3", "Retrieval views"));
  nav.append(button("Text retrieval", () => openLexical()));
  nav.append(button("Vector inspection",()=>openVectors()));
  nav.append(el("p","Stored graph and history: open a canonical record.",{class:"muted"}));
  nav.append(button("Compare observations",openComparison));
  for(const side of ["A","B"])if(pins[side].id)nav.append(button(`Unpin ${side}`,()=>unpin(side)));
  layout.append(nav, panel);
  root.append(layout);
  actions(panel);
  return { nav, panel };
}
function renderInventory() {
  const data = view.data,
    { nav, panel } = shell();
  updateDiscoveredScopes(data);
  panel.append(el("h2", "Database inventory"));
  if (isMultiScope() || activeScope !== null) {
    const switcherSection = el("section", null, {
      class: "scope-switcher",
      "aria-label": "Scope switcher",
    });
    const scopeField = el("label", "Active scope");
    const scopeSelect = buildScopeSelect("Active scope", activeScope, false);
    scopeSelect.addEventListener("change", () =>
      run(() => inventory(decodeScopeOption(scopeSelect.value))),
    );
    scopeField.append(scopeSelect);
    switcherSection.append(scopeField);
    if (discoveredScopes.length) {
      const list = el("ul", null, { class: "scope-summary-list muted" });
      for (const entry of discoveredScopes) {
        list.append(
          el(
            "li",
            `${scopeLabel(entry.scope)}: ${entry.total} visible (${entry.node} nodes, ${entry.edge} edges, ${entry.current} current, ${entry.log} log)`,
          ),
        );
      }
      switcherSection.append(list);
    }
    panel.append(switcherSection);
  }
  metadata(panel, data.database);
  const counts = el("dl");
  for (const [k, v] of Object.entries(data.counts)) {
    const unit =
      k === "current"
        ? "current values"
        : k === "log"
          ? "retained mutation entries"
          : k === "projection"
            ? "materialized attribute entries"
            : k === "commit_timestamps"
              ? "commit timestamp entries"
              : "canonical revisions";
    const category={current:"Operational",log:"Operational",projection:"Projection",commit_timestamps:"History",node:"Node",edge:"Edge"}[k]??k;
    const extra=k==="projection"&&v.property_value_entries!==undefined?`; property value entries: ${v.property_value_entries}`:"";
    counts.append(
      el("dt", `${category} ${unit}`),
      el(
        "dd",
        `${v.revisions ?? v.values ?? v.entries ?? v.attribute_entries} (${v.exact ? "exact" : "partial"}); logical identities: ${v.logical_identities ?? "not applicable / unknown"}${extra}`,
      ),
    );
  }
  panel.append(counts, el("h3", "Operational collections"));
  for (const c of data.collections) {
    const label = `Collection ${c.name} (${c.mode})`;
    const b = button(label, () =>
      page(c.mode === "latest_state" ? "current" : "log", {
        collection: c.name,
        ...(activeScope !== null ? { scope: activeScope } : {}),
      }),
    );
    b.disabled = c.name_complete === false;
    panel.append(
      b,
      el("p", c.engine_owned ? "Engine-owned bookkeeping" : "Registered operational data", {
        class: "muted",
      }),
    );
    const navigation = button(label, () =>
      page(c.mode === "latest_state" ? "current" : "log", {
        collection: c.name,
        ...(activeScope !== null ? { scope: activeScope } : {}),
      }),
    );
    navigation.disabled = c.name_complete === false;
    nav.append(navigation);
    if (c.name_complete === false)
      panel.append(
        el("p", "Collection name is a bounded preview; exact lookup is unavailable.", {
          class: "muted",
        }),
      );
  }
  panel.append(el("h3", "Projection declarations"));
  if (!data.projections.length)
    panel.append(
      el(
        "p",
        data.scope.projections_complete
          ? "No projection declarations configured."
          : "Projection declarations were not fully enumerated; this response does not prove none are configured.",
      ),
    );
  for (const p of data.projections)
    panel.append(
      button(`Projection ${p.name}`, () => {
        navigation++;
        remember();
        view = { kind: "projection", data: p, source: data };
        render();
      }),
    );
  panel.append(el("h3", "Scoped kind and collection counts"));
  metadata(panel, data.groups);
  scope(panel, data);
  bind(data, () => {
    if (view?.data === data) view.data = null;
  });
}
function renderProjection() {
  const { panel } = shell();
  panel.append(el("h2", "Projection declaration"));
  metadata(panel, view.data);
  panel.append(
    el(
      "p",
      "Stored scalar attribute materialization does not prove body-field absence or vector readiness. Declarations and observed materialization are separate.",
    ),
  );
  if (view.data.entity_class === "edge")
    panel.append(
      el(
        "p",
        "Edge property declarations materialize into canonical property values; inspect canonical edges to review per-edge values.",
      ),
    );
  else if (view.data.name_complete !== false)
    panel.append(
      button("Browse verified attribute origins", () =>
        page("node", {
          attribute: view.data.name,
          ...(activeScope !== null ? { scope: activeScope } : {}),
        }),
      ),
    );
  else
    panel.append(
      el(
        "p",
        "Declaration name is a bounded preview. Exact origin lookup is unavailable from a truncated name.",
      ),
    );
}
async function page(className, filter, after = "0", sample = false) {
  const current = view, focus = document.activeElement, token = ++navigation;
  const data = await api({
    op: sample ? "sample" : "page",
    class: className,
    filter,
    after,
    ...(sample ? {} : { limit: 50 }),
  }, true, () => token === navigation);
  remember(current, focus);
  view = { kind: "page", data, className, filter, after, sample, selected: null };
  render();
}
function renderPage() {
  const current = view,
    { panel } = shell();
  scrubDOM(current.workspace);current.workspace=panel;
  pinControls(panel,current,current.data);
  panel.append(
    el(
      "h2",
      current.sample
        ? `${current.className} observed-field sample`
        : `${current.className} records`,
    ),
    button("Sample observed fields", () => page(current.className, current.filter, "0", true)),
  );
  if (current.sample)
    panel.append(
      el(
        "p",
        "Deterministic first matching retained records. Observed content fields do not prove field absence or registered schema.",
      ),
    );
  const filters = el("form", null, { class: "filters" });
  const supported = [
    "identity",
    ...(current.className === "node"
      ? ["kind", "role", "source", "attribute"]
      : current.className === "edge"
        ? ["kind", "source"]
        : []),
  ];
  const inputs = {};
  let scopeSelect = null;
  if (isMultiScope() || current.filter.scope !== undefined) {
    const scopeField = el("label", "Scope filter");
    scopeSelect = buildScopeSelect("Scope filter", current.filter.scope ?? null, false);
    scopeField.append(scopeSelect);
    filters.append(scopeField);
  }
  for (const key of supported) {
    const [label, input] = labelInput(
      `${key[0].toUpperCase() + key.slice(1)} filter`,
      current.filter[key] ?? "",
    );
    filters.append(label);
    inputs[key] = input;
  }
  const apply = el("button", "Apply filters", { type: "submit" });
  filters.append(apply);
  filters.addEventListener("submit", (event) => {
    event.preventDefault();
    run(() => {
      const filter = {
        ...(current.filter.collection ? { collection: current.filter.collection } : {}),
      };
      if (scopeSelect) {
        const chosenScope = decodeScopeOption(scopeSelect.value);
        activeScope = chosenScope;
        if (chosenScope !== null) filter.scope = chosenScope;
      }
      for (const [k, input] of Object.entries(inputs)) if (input.value) filter[k] = input.value;
      return page(current.className, filter, "0", current.sample);
    });
  });
  if (current.filter.attribute)
    panel.append(
      el(
        "p",
        "Stored scalar attribute materialization does not prove body-field absence or vector readiness.",
        { class: "muted" },
      ),
    );
  panel.append(
    filters,
    el(
      "p",
      "Identity lookup is exact within the selected object class. Arbitrary predicates are unavailable.",
      { class: "muted" },
    ),
  );
  const list = el("div", null, { class: "record-list", role: "list" });
  if (!current.data.items.length)
    list.append(
      el(
        "p",
        current.data.status === "complete"
          ? "No records in this scope."
          : "No matches in the examined range; incomplete work or classification cannot prove absence.",
      ),
    );
  for (const item of current.data.items) {
    const operational = item.class === "current" || item.class === "log";
    const name = operational
      ? `${item.collection_name} / ${item.record_key ?? "(key withheld or unavailable)"}${item.class === "log" ? ` / entry ${item.entry_id}` : ""}`
      : (item.logical_id ?? `${item.class} cursor ${item.write_cursor}`);
    const b = button(
      `Inspect ${name}`,
      async () => {
        current.selected = item.handle;
        await openDetail(item.handle);
      },
      { "data-handle": item.handle },
    );
    if (current.sample) {
      const fields = el("section", null, { "aria-label": `Observed fields for ${name}` });
      const observation = item.field_observation;
      fields.append(
        el(
          "p",
          `Content sample: ${observation.state}; root kind: ${observation.root_kind ?? "not inferred"}; complete for this record: ${observation.complete_for_this_record}`,
        ),
      );
      for (const field of observation.names) fields.append(el("p", field));
      list.append(fields);
    }
    const preview=item.display_preview;
    recordDisplay(list,preview);
    const scopeTag =
      isMultiScope() || (item.scope !== undefined && item.scope !== "")
        ? ` · scope ${item.scope === "" ? '""' : item.scope}`
        : "";
    list.append(
      b,
      el(
        "span",
        `${item.kind ?? item.mode}${scopeTag} · ${item.payload_status} · cursor ${item.write_cursor}`,
        { class: "badge" },
      ),
    );
  }
  panel.append(list);
  if (current.data.scope.continuation)
    panel.append(
      button("Next page", () =>
        page(current.className, current.filter, current.data.scope.continuation, current.sample),
      ),
    );
  scope(panel, current.data);
  const owner = current.data;
  bind(owner, () => {
    if (current.data !== owner) return;
    current.data = null;
    current.returnFocus?.replaceChildren();
    current.returnFocus = null;
  });
  if (current.selected)
    queueMicrotask(() => root.querySelector(`[data-handle="${current.selected}"]`)?.focus());
}
async function copyField(handle, field, start = 0, end = null, decodeString = false) {
  const version = cacheVersion;
  let offset = start,
    text = "",
    bytes = 0,
    nullValue = false;
  do {
    const n = end === null ? 262144 : Math.min(262144, end - offset);
    if (n <= 0) break;
    const data = await api({ op: "chunk", handle, field, offset: String(offset), limit: n });
    if (data.state === "null") {
      text = "null";
      nullValue = true;
      break;
    }
    bytes += new TextEncoder().encode(data.text).length;
    if (bytes > 20 * 1024 * 1024) throw Error("copy_budget_exhausted");
    text += data.text;
    if (!data.continuation) break;
    offset = Number(data.continuation);
  } while (end === null || offset < end);
  if (!(await validate({ kind: "detail", handle })) || version !== cacheVersion)
    throw Error("reference_expired");
  if (decodeString) {
    text = JSON.parse(text);
    if (
      typeof text !== "string" ||
      /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(text)
    )
      throw Error("unsupported_value_encoding");
  }
  await navigator.clipboard.writeText(text);
  say(nullValue ? "Copied null as JSON null." : "Copied exact stored value.");
}
async function tree(parent, handle, start = 0, accept = () => true, field = "body") {
  const data = await api({ op: "tree", handle, field, start: String(start), after: null }, true, accept);
  drawTree(parent, handle, data, accept, data.field ?? field);
}
function drawTree(parent, handle, data, accept, field = "body") {
  const targetField = data.field ?? field ?? "body";
  const list = el("ul", null, { class: "tree", role: "tree" });
  for (const item of data.items) {
    const li = el("li", null, { role: "treeitem" }),
      name = item.key ?? String(item.index);
    li.append(el("span", `${name}: `));
    if (item.expandable) {
      let expanded = false;
      const contents = el("div"),
        toggle = button(
          `Expand ${name}`,
          async () => {
            if (expanded) {
              contents.replaceChildren();
              toggle.textContent = `Expand ${name}`;
              toggle.setAttribute("aria-expanded", "false");
              expanded = false;
            } else {
              await tree(contents, handle, Number(item.start), () => accept() && !data.evicted, targetField);
              toggle.textContent = `Collapse ${name}`;
              toggle.setAttribute("aria-expanded", "true");
              expanded = true;
            }
          },
          { "aria-expanded": "false" },
        );
      li.append(toggle, el("span", item.kind, { class: "badge" }), contents);
    } else
      li.append(
        el("span", item.lexeme, { class: "value" }),
        item.preview_truncated
          ? el("span", " (preview; copy exact range)", { class: "muted" })
          : el("span", ""),
      );
    li.append(
      button(`Copy value ${name}`, () =>
        copyField(handle, targetField, Number(item.start), Number(item.end), item.kind === "string"),
      ),
      button(`Copy raw JSON ${name}`, () =>
        copyField(handle, targetField, Number(item.start), Number(item.end)),
      ),
    );
    list.append(li);
  }
  parent.append(list);
  if (data.continuation)
    parent.append(
      button("More structured fields", async () => {
        const next = await api({ op: "tree", handle, field: targetField, start: data.start, after: data.continuation }, true, () => accept() && !data.evicted);
        drawTree(parent, handle, next, accept, targetField);
      }),
    );
  bind(data, () => {
    list.replaceChildren(
      el("li", "Structured observation evicted. Collapse and expand explicitly to read again."),
    );
  });
}
function renderDetail() {
  const current = view,
    { panel: host } = shell();
  if (current.workspace) {
    current.workspace.firstElementChild.replaceWith(host.firstElementChild);
    host.replaceWith(current.workspace);
    return;
  }
  const panel = host;
  panel.classList.add("record-workspace");
  panel.append(button("Refresh this view", refreshView));
  current.workspace = panel;
  const owner = current.data;
  const ownsWorkspace = () => current.data === owner && !owner.evicted && current.workspace === panel;
  const field = current.field ?? current.data.selected_field ?? "body";
  panel.append(el("h2", field === "body" ? "Record detail" : `Full field: ${field}`));
  const preview=current.data.record.display_preview;
  recordDisplay(panel,preview);
  if (current.data.record.scope !== undefined) {
    const scopeVal = current.data.record.scope;
    panel.append(
      el(
        "p",
        `Scope: ${scopeVal === "" ? '"" (empty / default namespace)' : scopeVal === null ? "SQL NULL" : String(scopeVal)}`,
        { class: "record-scope muted" },
      ),
    );
  }
  const buttons = el("div", null, { class: "actions" });
  if (current.data.record.class === "attribute")
    buttons.append(
      button("Copy exact identity", () => copyField(current.handle, "attribute_name")),
    );
  else if (current.data.record.logical_id !== null && current.data.record.logical_id !== undefined)
    buttons.append(button("Copy exact identity", () => copyField(current.handle, "logical_id")));
  else if (current.data.record.record_key !== undefined)
    buttons.append(button("Copy exact record key", () => copyField(current.handle, "record_key")));
  if (current.data.record.class === "log")
    buttons.append(button("Copy exact entry ID", () => copyField(current.handle, "entry_id")));
  buttons.append(button("Copy content", () => copyField(current.handle, field)));
  if (["node", "edge", "attribute"].includes(current.data.record.class))
    buttons.append(button("Indexed text", () => openLexical(current.handle)),button("Stored vectors",()=>openVectors(current.handle)));
  if (["node","edge"].includes(current.data.record.class)) buttons.append(button("Stored graph",()=>openGraph(current.handle)),button("Provenance and history",()=>openInvestigation(current.handle)));
  if (field === "body" && current.data.record.retained_erasure_pending && pair.policy.show_erased) {
    const [label, destination] = labelInput("Local export destination");
    panel.append(label);
    buttons.append(
      button("Export retained content", async () => {
        const result = await api({
          op: "export",
          handle: current.handle,
          destination: destination.value,
        });
        say(
          result.status === "complete"
            ? "Retained-content export complete."
            : `Export ${result.status}; the local file may be incomplete.`,
        );
      }),
    );
  }
  const facts = el("section", null, { class: "record-facts", "aria-label": "Observed record facts" });
  facts.append(el("h3", "Observed record facts"));
  const originalFacts=el("div");
  const f=current.data.facts;
  facts.append(el("p",`Storage: ${f.storage.state} · ${f.storage.representation.replaceAll("_"," ")}; inspection: ${f.eligibility.inspection}.`),el("p",`Representation read: ${f.readiness.representation_read}; retrieval readiness and eligibility: ${f.readiness.retrieval} / ${f.eligibility.retrieval}.`),el("p",`Session dense runtime: ${f.runtime.dense_query}; these inspection facts do not establish selection by an earlier query.`),el("p",`Read source: ${f.storage.source}; read time: ${epochUTC(current.data.context.read_started_epoch_ms,"milliseconds")??"unknown"}.`));
  for (const [name, fact] of Object.entries(f)) { originalFacts.append(el("h4",name));metadata(originalFacts,fact); }
  exactEvidence(facts,"Exact observed record facts",originalFacts);
  const identityHeading=el("h3","Identity and system metadata");
  panel.append(buttons, identityHeading);
  metadata(panel, current.data.record, current);
  panel.append(facts);
  if (current.data.record.class === "node" || current.data.record.class === "edge")
    panel.append(
      el(
        "p",
        current.data.record.revision_id === null
          ? "No recorded revision evidence"
          : "Revision evidence: recorded",
      ),
    );
  const contentHeading=el("h3",field === "body" ? "Stored content" : `Full stored field: ${field}`);
  panel.append(contentHeading);
  const content = current.data.content;
  const display = el(
    "pre",
    content.state === "null" ? "null" : content.state === "empty" ? "Empty string" : content.text,
    { class: "content" },
  );
  const chunkContext = el("pre", JSON.stringify(current.data.context, null, 2), { class: "chunk-context" });
  panel.append(display, chunkContext);
  let displayedObservation = current.data;
  function showChunk(part) {
    if (!current.data) return;
    displayedObservation = part;
    display.textContent = part.state === "null" ? "null" : part.state === "empty" ? "Empty string" : part.text;
    chunkContext.textContent = JSON.stringify(part.context, null, 2);
    bind(part, () => {
      if (displayedObservation !== part) return;
      display.textContent = "Chunk observation evicted. Refresh explicitly to read again.";
      chunkContext.textContent = "";
      back.disabled = true;
      forward.disabled = true;
    });
  }
  const paging = el("div", null, { class: "actions" });
  let next = content.continuation,
    offset = 0;
  const previous = [];
  const back = button("Previous content chunk", async () => {
    if (!previous.length) return;
    next = String(offset);
    offset = previous.pop();
    const part = await api({
      op: "chunk",
      handle: current.handle,
      field,
      offset: String(offset),
      limit: 65536,
    }, true, ownsWorkspace);
    showChunk(part);
    back.disabled = previous.length === 0;
    forward.disabled = next === null;
  });
  const forward = button("Next content chunk", async () => {
    if (next === null) return;
    previous.push(offset);
    offset = Number(next);
    const part = await api({
      op: "chunk",
      handle: current.handle,
      field,
      offset: String(offset),
      limit: 65536,
    }, true, ownsWorkspace);
    showChunk(part);
    next = part.continuation;
    back.disabled = previous.length === 0;
    forward.disabled = next === null;
  });
  back.disabled = true;
  forward.disabled = next === null;
  paging.append(back, forward);
  panel.append(paging);
  const structured = el("section", null, { "aria-label": "Structured content" });
  panel.append(structured);
  if (
    (field === "body" || field === "properties") &&
    current.data.record.class !== "attribute" &&
    /^[\s]*[\[{]/.test(content.text)
  )
    run(async () => {
      structured.append(el("h3", "Structured values"));
      await tree(structured, current.handle, 0, ownsWorkspace, field);
    });
  let propertiesSection = null;
  if (field === "body" && ["node", "edge"].includes(current.data.record.class)) {
    const rec = current.data.record;
    propertiesSection = el("section", null, {
      class: "record-properties",
      "aria-label": "Properties",
    });
    propertiesSection.append(el("h3", "Properties"));
    if (rec.properties_status === "refused") {
      propertiesSection.append(
        el("p", "refused (erasure-pending)", { class: "properties-status muted" }),
      );
    } else if (rec.properties_status === "available") {
      if (rec.properties_bytes === null || rec.properties_bytes === undefined || rec.properties === null) {
        propertiesSection.append(el("p", "null", { class: "properties-null" }));
      } else {
        propertiesSection.append(
          el(
            "p",
            `Stored bytes: ${rec.properties_bytes} · storage type: ${rec.properties_storage ?? "unknown"}`,
            { class: "properties-meta muted" },
          ),
        );
        const propActions = el("div", null, { class: "actions" });
        propActions.append(
          button("Open full properties", () => openField(current, "properties")),
          button("Copy full properties", () => copyField(current.handle, "properties")),
        );
        propertiesSection.append(propActions);
        const propText = String(rec.properties);
        propertiesSection.append(
          el("pre", propText === "" ? "Empty string" : propText, { class: "properties-preview" }),
        );
        const shown = new TextEncoder().encode(propText).length;
        const total = Number(rec.properties_bytes);
        if (shown < total) {
          propertiesSection.append(
            el(
              "p",
              `Preview: ${shown} of ${total} stored bytes; open or copy the full properties.`,
              { class: "muted" },
            ),
          );
        }
        if (total <= 16384 && /^[\s]*[\[{]/.test(propText)) {
          const propTree = el("div", null, {
            class: "properties-tree",
            "aria-label": "Structured properties",
          });
          propertiesSection.append(propTree);
          run(async () => {
            await tree(propTree, current.handle, 0, ownsWorkspace, "properties");
          });
        }
      }
    }
    panel.append(propertiesSection);
  }
  const associations = el("section", null, { "aria-label": "Recorded associations" });
  panel.append(associations);
  if (current.data.record.class === "attribute") {
    associations.append(el("h3", "Verified canonical origin"));
    metadata(associations, current.data.record.origin);
    associations.append(button("Open verified canonical origin", async () => {
      await openDetail(current.data.origin_reference.handle);
    }));
  } else if (field === "body") {
    if (["node", "edge"].includes(current.data.record.class))
      run(() => renderAssociations(associations, current));
    else
      associations.append(
        el(
          "p",
          "Canonical source and attribute links are not applicable to this operational record.",
        ),
      );
  }
  for(const node of [facts,contentHeading,display,paging,structured,...(propertiesSection?[propertiesSection]:[]),associations])panel.insertBefore(node,identityHeading);
  scope(panel, current.data);
  bind(owner, () => {
    if (current.data !== owner) return;
    current.data = null;
    scrubDOM(current.workspace);
    current.workspace = null;
    current.associations = null;
    current.returnFocus = null;
    if (view === current) queueMicrotask(()=>{if(view===current)render();});
  });
}
await run(async () => {
  const response = await fetch("/handshake");
  if (!response.ok) throw Error("local_access_denied");
  const info = await response.json();
  if (info.build !== BUILD || info.protocol !== 1) throw Error("expired");
  initialPath = info.path ?? "";
  pair = {
    build: info.build,
    protocol: info.protocol,
    launch: info.launch,
    attachment: info.attachment,
  };
  pair.policy = info.policy;
  const policy = pair.policy;
  delete pair.policy;
  Object.defineProperty(pair, "policy", { value: policy, enumerable: false });
  selector();
  attachmentStatus();
});

async function renderAssociations(parent, current) {
  const owner = current.data;
  const data = await api({ op: "associations", handle: current.handle }, true, () => current.data === owner && !owner.evicted);
  if (current.data !== owner || owner.evicted) return;
  current.associations = data;
  parent.append(el("h3", "Recorded object links"));
  for (const link of data.links) {
    const label = link.kind.replaceAll("_", " ");
    parent.append(
      el(
        "p",
        `${label}: ${link.resolution}; visible targets ${link.cardinality}${link.truncated ? " (partial)" : ""}`,
        { class: "muted" },
      ),
    );
    for (const target of link.targets) {
      const name =
        target.logical_id ?? target.revision_id ?? `${target.class} cursor ${target.write_cursor}`;
      const descriptionId = `associated-target-${target.handle}`;
      const description = el(
        "span",
        `${target.class} · ${target.revision_id ? `recorded revision ${target.revision_id}` : "no recorded revision evidence"} · internal write cursor ${target.write_cursor}`,
        { id: descriptionId, class: "badge" },
      );
      parent.append(
        button(
          `Open ${label} ${name}`,
          async () => {
            await openDetail(target.handle);
          },
          { "aria-describedby": descriptionId },
        ),
        description,
      );
    }
  }
  parent.append(el("h3", "Materialized attributes"));
  if (!data.attributes.length)
    parent.append(
      el(
        "p",
        data.attributes_complete
          ? "No verified attribute materializations for this revision."
          : "Materializations are not fully covered by this response budget.",
      ),
    );
  for (const attribute of data.attributes) {
    if (attribute.handle)
      parent.append(
        button(`Open materialized attribute ${attribute.name}`, async () => {
          await openDetail(attribute.handle);
        }),
      );
    parent.append(
      el("p", attribute.name),
      el("pre", attribute.value === null ? "null" : attribute.value, { class: "attribute-value" }),
      el(
        "p",
        `Stored bytes: ${attribute.total_bytes ?? "null"}; preview limit: ${attribute.preview_limit}`,
        { class: "muted" },
      ),
    );
  }
  parent.append(el("h3", "Materialized property values"));
  const propValues = data.property_values ?? [];
  if (!propValues.length)
    parent.append(
      el(
        "p",
        data.property_values_complete !== false
          ? "No verified property-value materializations for this revision."
          : "Property-value materializations are not fully covered by this response budget.",
      ),
    );
  for (const pv of propValues) {
    const entry = el("div", null, { class: "property-value-entry" });
    entry.append(
      el("p", `${pv.name} (${pv.value_type} · ${pv.entity_class})`),
      el(
        "pre",
        pv.value === null || pv.value === undefined
          ? "null"
          : pv.value === ""
            ? "Empty string"
            : String(pv.value),
        { class: "property-value" },
      ),
      el(
        "p",
        `Stored bytes: ${pv.total_bytes ?? "null"}; preview limit: ${pv.preview_limit}`,
        { class: "muted" },
      ),
    );
    parent.append(entry);
  }
  parent.append(el("h3", "Declarations and observed materialization"));
  for (const projection of data.projections) metadata(parent, projection);
  parent.append(el("p", "Association facts use a separate read observation.", { class: "muted" }));
  scope(parent, data);
  bind(data, () =>
    parent.replaceChildren(el("p", "Association observation evicted; reopen explicitly.")),
  );
}

setInterval(() => {
  if (!pair?.attachment || polling || root.getAttribute("aria-busy") === "true") return;
  polling = true;
  const current = view, token = navigation;
  run(() => validate(current, () => current === view && token === navigation)).finally(() => {
    polling = false;
  });
}, 500);


async function openLexical(handle = null) {
  const current = view, focus = document.activeElement, token = ++navigation;
  const source = await api(handle
    ? {op:"indexed",handle,offset:"0",limit:4096}
    : {op:"catalog"}, true, () => token === navigation);
  const declarations=handle?await api({op:"catalog"},true,()=>token===navigation):source;
  updateDiscoveredScopes(declarations);
  remember(current,focus);
  view = {kind:"lexical",handle,data:source,source,declarations,request:{route:"body",query:"",property:null,filter:{},view:{},...(activeScope!==null?{scope:activeScope}:{}),limit:10}};
  render();
}
function highlightNative(parent, snippet) {
  const bytes = new TextEncoder().encode(snippet.text), decoder = new TextDecoder("utf-8",{fatal:true});
  let offset = 0;
  for (const [start,end] of snippet.highlights) {
    if (!Number.isInteger(start) || !Number.isInteger(end) || start < offset || end <= start || end > bytes.length)
      throw Error("unsupported_value_encoding");
    parent.append(document.createTextNode(decoder.decode(bytes.slice(offset,start))),el("mark",decoder.decode(bytes.slice(start,end))));
    offset = end;
  }
  parent.append(document.createTextNode(decoder.decode(bytes.slice(offset))));
}
async function copyLexical(current,text) {
  const version = cacheVersion, token = navigation;
  const accept = () => view === current && token === navigation;
  if (!(await validate(current,accept)) || version !== cacheVersion || !accept()) return;
  await navigator.clipboard.writeText(text);
  say("Observed text copied exactly; displayed completeness and observation scope apply.");
}
function renderLexical() {
  const current = view, {panel:host} = shell();
  if (current.workspace) {
    current.workspace.firstElementChild.replaceWith(host.firstElementChild);
    host.replaceWith(current.workspace);
    return;
  }
  const panel=host, owner=current.data;
  current.workspace=panel;
  panel.classList.add("lexical-workspace");
  panel.append(el("h2","Text retrieval"),button("Refresh this view",refreshView));
  const indexed=el("section",null,{"aria-label":"Stored indexed text"});
  indexed.append(el("h3","Stored indexed text"));
  if(current.source?.evicted)indexed.append(el("p","Original source observation is no longer retained; the result below keeps its own original context."));
  if (current.handle && !current.source?.evicted) {
    const parts=current.source?.items ?? [];
    if (!parts.length) indexed.append(el("p","No stored lexical materialization observed in this read; retrieval readiness and eligibility remain unevaluated."));
    for (const part of parts) {
      const section=el("section",null,{class:"indexed-part"});
      section.append(el("h4",part.key),el("pre",part.text,{class:"indexed-content"}),el("p",`${part.offset} of ${part.total_bytes ?? "null"} bytes; complete ${part.complete}. ${part.evidence}`));
      section.append(el("pre",JSON.stringify({index:part.index,field:part.field,configured_tokenizer:part.configured_tokenizer,effective_tokenizer:part.effective_tokenizer,effective_ddl:part.effective_ddl},null,2)));
      const controls=el("div",null,{class:"actions"});
      controls.append(button(`Copy indexed window ${part.key}`,()=>copyLexical(current,part.text)));
      if (part.continuation) controls.append(button(`Next indexed window ${part.key}`,async()=>{
        const token=++navigation;
        const observation=await api({op:"indexed",handle:current.handle,key:part.key,offset:part.continuation,limit:65536},true,()=>token===navigation&&view===current);
        current.source=observation;current.workspace=null;render();
      }));
      section.append(controls);indexed.append(section);
    }
    indexed.append(el("pre",JSON.stringify(current.source.context,null,2),{class:"indexed-context"}));
  } else indexed.append(el("p","Select Indexed text from an exact record to inspect its stored representations."));
  panel.append(indexed);
  const form=el("form",null,{"aria-label":"Native lexical query"});
  const routeLabel=el("label","Lexical route"),route=el("select",null,{"aria-label":"Lexical route"});
  for (const [value,name] of [["body","Body and edge text"],["property","Named property text"],["bm25f","BM25F substrate"]]) {
    const option=el("option",name,{value});route.append(option);
  }
  route.value=current.request.route;routeLabel.append(route);form.append(routeLabel);
  let scopeSelect=null;
  if(isMultiScope()||current.request.scope!==undefined){
    const scopeField=el("label","Scope filter");
    scopeSelect=buildScopeSelect("Scope filter",current.request.scope??null,true);
    scopeField.append(scopeSelect);form.append(scopeField);
  }
  const make=(label,value="")=>{const [node,input]=labelInput(label,value ?? "");form.append(node);return input;};
  const choices=propertyChoices(current.declarations??(current.source?.projections?current.source:null));
  const property=make("Property name",current.request.property ?? choices[0] ?? "");
  const options=el("datalist",null,{id:"declared-properties"});for(const name of choices)options.append(el("option",null,{value:name}));property.setAttribute("list","declared-properties");form.append(options);
  const queryFeedback=el("div",null,{role:"status","aria-label":"Lexical query feedback"});if(current.queryFeedback)queryFeedback.append(el("p",current.queryFeedback));
  form.append(el("p",choices.length?`Declared searchable properties: ${choices.join(", ")}${current.declarations?.scope?.projections_complete===false?" (partial declaration listing)":""}`:"No exact property FTS declaration available; refresh declarations explicitly.",{class:"muted"}),queryFeedback);
  property.disabled=route.value!=="property";
  const query=make("Query text",current.request.query);
  const limit=make("Result limit",String(current.request.limit));limit.type="number";limit.min="1";limit.max="100";
  const filterInputs={};
  for (const [key,label] of [["kind","Kind filter"],["source_type","Source type filter"],["status","Status filter"],["created_after","Created after epoch seconds"]])
    filterInputs[key]=make(label,current.request.filter[key]);
  const attribute=make("Filter attribute name",current.request.filter.attributes?.[0]?.[0]);
  const attributeValue=make("Filter attribute value",current.request.filter.attributes?.[0]?.[1]);
  const viewInputs={};
  for (const [key,label] of [["include_superseded","Include superseded in query"],["include_inactive","Include inactive in query"],["include_out_of_window","Include outside validity window in query"]]) {
    const node=el("label",label),input=el("input",null,{type:"checkbox"});input.checked=!!current.request.view[key];node.append(input);form.append(node);viewInputs[key]=input;
  }
  const validAt=make("Valid at epoch seconds",current.request.view.valid_as_of);
  const bm25f=route.value==="bm25f";
  for (const input of [...Object.values(filterInputs),attribute,attributeValue,...Object.values(viewInputs),validAt]) input.disabled=bm25f;
  form.append(el("p",bm25f
    ? "Native BM25F: active/current nodes and native dependency eligibility; own-node validity windows ignored. No native filters/view/limit. Limit truncates native output. Query tokens are OR terms."
    : "Native plain-text query compiler. Query view is independent of stored inspection policy. Body scores use RRF; property scores use negated SQLite BM25. No missing-result cause is inferred."));
  function save() {
    const filter={},readView={};
    if (route.value!=="bm25f") {
      for (const [key,input] of Object.entries(filterInputs)) if(input.value!=="")filter[key]=input.value;
      if(attribute.value!=="" || attributeValue.value!=="")filter.attributes=[[attribute.value,attributeValue.value]];
      for (const [key,input] of Object.entries(viewInputs)) readView[key]=input.checked;
      if(validAt.value!=="")readView.valid_as_of=validAt.value;
    }
    const selectedScope=scopeSelect?decodeScopeOption(scopeSelect.value):(current.request.scope??null);
    current.request={route:route.value,query:query.value,property:route.value==="property"?property.value:null,filter,view:readView,...(selectedScope!==null?{scope:selectedScope}:{}),limit:Number(limit.value)};
  }
  form.addEventListener("input",save);
  if(scopeSelect)scopeSelect.addEventListener("change",save);
  route.addEventListener("change",()=>{save();current.workspace=null;render();});
  async function execute() {
    save();
    if(current.request.route==="property"&&!choices.includes(property.value)){
      current.queryFeedback=`Choose a declared searchable property: ${choices.join(", ")||"none available; refresh declarations"}.`;
      queryFeedback.replaceChildren(el("p",current.queryFeedback));return;
    }
    if(!query.value.trim()){
      current.queryFeedback="Operation did not complete (invalid_range); enter query text. Previous results retain their original request.";
      queryFeedback.replaceChildren(el("p",current.queryFeedback));return;
    }
    const token=++navigation;
    let data;try{data=await api({op:"lexical",...current.request},true,()=>token===navigation&&view===current);}
    catch(error){if(error.silent||token!==navigation||view!==current)return;current.queryFeedback=`Operation did not complete (${error.message}); check the selected property and request. Previous results retain their original request.`;if(queryFeedback.isConnected)queryFeedback.replaceChildren(el("p",current.queryFeedback));else render();return;}
    current.queryFeedback=null;
    scrubDOM(current.workspace);current.returnFocus=null;
    current.data=data;current.workspace=null;render();
    say(data.items.length ? `Returned ${data.items.length} permitted native matches; original ranks preserved.` : "No permitted matches in this bounded native result. Absence cause is not established.");
  }
  form.append(button("Run lexical query",execute,{type:"button"}));
  form.addEventListener("submit",event=>{event.preventDefault();run(execute);});
  panel.insertBefore(form,indexed);
  if (current.data.query_context) {
    const results=el("section",null,{class:"lexical-results","aria-label":"Lexical results"});
    observationSummary(results,current.data);
    pinControls(results,current,current.data);
    results.append(el("h3",`Native results: ${current.data.scope.score_family} (${current.data.scope.score_direction})`));
    if (!current.data.items.length)results.append(el("p","No permitted matches in this bounded native observation."));
    for (const hit of current.data.items) {
      const card=el("article",null,{class:"lexical-hit"});
      const name=hit.native_id?.value ?? hit.origin.targets[0]?.record.logical_id ?? `cursor ${hit.write_cursor}`;
      card.append(el("h4",`Rank ${hit.rank}: ${name}`),el("p",`${current.data.scope.score_family}: ${hit.score}; ${hit.class}; ${hit.representative_branch}`));
      const originActions=el("div",null,{class:"actions"});card.append(originActions);
      card.append(el("pre",hit.payload.text,{class:"observed-payload"}),el("p",`Original/later payload scope: ${hit.payload.scope}; ${hit.payload.total_bytes ?? "null"} bytes; complete ${hit.payload.complete}`));
      originActions.append(button(`Copy observed text ${hit.rank}`,()=>copyLexical(current,hit.payload.text)));
      if (hit.snippet.state==="available") {
        const snippet=el("pre",null,{class:"native-snippet"});highlightNative(snippet,hit.snippet);card.append(snippet);
        card.append(el("p",`Engine snippet anchor ${hit.snippet.anchor}; cut ${hit.snippet.cut}; ${hit.snippet.scope}; half-open UTF8 byte ranges`));
        card.append(el("pre",JSON.stringify({windows:hit.snippet.windows,spans:hit.snippet.spans,highlights:hit.snippet.highlights},null,2)));
        card.append(button(`Copy observed snippet ${hit.rank}`,()=>copyLexical(current,hit.snippet.text)));
      } else card.append(el("p","no_native_snippet: match spans unavailable for this native route."));
      card.append(el("p",`Origin: ${hit.origin.state}; ${hit.origin.scope}. Record identity: ${hit.origin.all_equal_body_contributors}.`));
      for (const origin of hit.origin.targets)
        originActions.append(button(`Open exact origin ${origin.record.logical_id ?? origin.record.write_cursor}`,()=>openDetail(origin.handle)));
      results.append(card);
    }
    results.append(el("h3","Native query observation"),el("pre",JSON.stringify({request:current.data.request,...current.data.query_context},null,2),{class:"query-context"}),el("h3","Later policy, origin and index observation"),el("pre",JSON.stringify({context:current.data.context,metadata:current.data.metadata,admission:current.data.admission},null,2)));
    panel.insertBefore(results,indexed);
    scope(panel,current.data);
  }
  if(current.declarations&&current.declarations!==owner)bind(current.declarations,()=>{current.declarations=null;scrubDOM(current.workspace);current.workspace=null;if(view===current)queueMicrotask(()=>{if(view===current)render();});});
  bind(owner,()=>{
    if(current.data!==owner)return;
    scrubDOM(current.workspace);current.workspace=null;
    if(view===current)queueMicrotask(()=>{if(view===current)render();});
  });
}

async function openVectors(handle=null){
  const current=view,focus=document.activeElement,token=++navigation;
  const source=await api(handle?{op:"vectors",handle}:{op:"catalog"},true,()=>token===navigation);
  if(!handle)updateDiscoveredScopes(source);
  remember(current,focus);view={kind:"vector",handle,source,data:source,nativeQuery:"",servingScope:activeScope};render();
}
function vectorOrigins(parent,item){
  const targets=item.origin?.targets??[];
  if(!targets.length)parent.append(el("p","No exact origin exposed by this observation."));
  if(targets.length>1)parent.append(el("p",`${targets.length} recorded origin choices`));
  for(const origin of targets){const name=origin.record.logical_id??origin.record.revision_id??origin.record.write_cursor;
    parent.append(button(`Open exact origin ${name}`,()=>openDetail(origin.handle)));
  }
}
function renderVector(){
  const current=view,{panel:host}=shell();
  if(current.workspace){current.workspace.firstElementChild.replaceWith(host.firstElementChild);host.replaceWith(current.workspace);return;}
  const panel=host;current.workspace=panel;panel.classList.add("vector-workspace");panel.append(el("h2","Vector inspection"));
  const stored=el("section",null,{"aria-label":"Stored vector representations"});stored.append(el("h3","Stored vector representations"));
  if(current.source?.evicted)stored.append(el("p","Original source observation is no longer retained; the result below keeps its own original context."));
  const representations=current.handle&&!current.source?.evicted?current.source.items:[];
  if(!representations?.length)stored.append(el("p","No stored vector materialization observed in this read."));
  async function execute(request){
    const token=++navigation,accept=()=>view===current&&navigation===token;
    const data=await api(request,true,accept);if(!accept())return;scrubDOM(current.workspace);current.returnFocus=null;current.data=data;current.lastRequest=request;current.workspace=null;render();
  }
  for(const item of representations??[]){
    const card=el("article",null,{class:"stored-vector"});card.append(el("h4",`Vector row ${item.vector_rowid} · window ${item.window.number??"unknown"}`));
    card.append(el("p",`${item.representation} · profile ${item.profile??"unknown"} · source cursor ${item.source_cursor} · ${item.materialization}`));
    metadata(card,item.producer);metadata(card,item.window);
    const values=el("details");values.append(el("summary",`${item.values?.length??0} stored f32 values`),el("pre",JSON.stringify(item.values,null,2),{class:"vector-values"}));card.append(values);
    card.append(button("Copy observed vector values",()=>copyLexical(current,JSON.stringify(item.values))));
    if(item.window.text!==null&&item.window.text!==undefined)card.append(el("h4","Recorded source window"),el("pre",item.window.text,{class:"vector-window"}));
    if(item.window.coverage==="unknown")card.append(el("p","Window byte end and coverage are unknown."));
    const find=button(`Find similar vector row ${item.vector_rowid}`,()=>execute({op:"neighborhood",handle:current.handle,vector_rowid:item.vector_rowid,after:"0",limit:10}));find.disabled=item.materialization!=="present";card.append(find);vectorOrigins(card,item);stored.append(card);
  }
  panel.append(stored);
  const runtime=current.source.scope?.runtime??{dense_query:current.source.database?.model_runtime==="available_cached_cpu"?"available":"unavailable",reason:current.source.database?.runtime_reason??"no_qualified_cached_runtime"};
  const serving=el("form",null,{"aria-label":"Native cached CPU serving"});serving.append(el("h3","Native cached CPU serving"));metadata(serving,runtime);
  let servingScopeSelect=null;
  if(isMultiScope()||current.servingScope!==null){
    const scopeField=el("label","Serving scope");
    servingScopeSelect=buildScopeSelect("Serving scope",current.servingScope,true);
    servingScopeSelect.addEventListener("change",()=>{current.servingScope=decodeScopeOption(servingScopeSelect.value);});
    scopeField.append(servingScopeSelect);serving.append(scopeField);
  }
  const [label,input]=labelInput("Semantic query text");input.maxLength=4096;input.value=current.nativeQuery;input.addEventListener("input",()=>{current.nativeQuery=input.value;});
  const servingRequest=()=>({op:"serving",query:input.value,limit:10,...(current.servingScope!==null?{scope:current.servingScope}:{})});
  const runQuery=button("Run native hybrid search",()=>execute(servingRequest()),{type:"button"});runQuery.disabled=runtime.dense_query!=="available";input.disabled=runQuery.disabled;if(servingScopeSelect)servingScopeSelect.disabled=runQuery.disabled;serving.append(label,runQuery);serving.addEventListener("submit",event=>{event.preventDefault();if(!runQuery.disabled)run(()=>execute(servingRequest()));});panel.append(serving);
  if(current.lastRequest)pinControls(panel,current,current.data);
  if(current.lastRequest?.op==="neighborhood"){
    const data=current.data,results=el("section",null,{class:"vector-results","aria-label":"Stored L2 neighborhood"});results.append(el("h3","Stored L2 neighborhood"),el("p","Stored uncentered f32 L2 distance · lower is nearer · nearest within the compared population"));
    const maximum=Math.max(1,...data.items.map(item=>Number(item.distance)));
    for(const item of data.items){const card=el("article",null,{class:"vector-hit"});const name=item.origin?.targets?.[0]?.record.logical_id??`vector row ${item.vector_rowid}`;
      card.append(el("h4",name),el("p",`Rank ${item.rank} · Distance: ${item.distance}`),el("meter",item.distance,{min:0,max:maximum,value:item.distance,"aria-label":`Stored L2 distance to ${name}: ${item.distance}`}));vectorOrigins(card,item);results.append(card);
    }
    if(!data.items.length)results.append(el("p","No policy-visible compatible vector was compared in this window."));
    results.append(el("pre",JSON.stringify({request:data.request,seed:data.seed,scope:data.scope,context:data.context},null,2),{class:"vector-context"}));panel.append(results);
  }else if(current.lastRequest?.op==="serving"){
    const data=current.data,results=el("section",null,{class:"serving-results","aria-label":"Native hybrid results"});results.append(el("h3","Native hybrid results"),el("p",`${data.scope.score_family} · native scores higher is better · dense stage distances ${data.scope.dense_stage_distances}`));
    for(const item of data.items){const card=el("article",null,{class:"serving-hit"});card.append(el("h4",item.native_id?.value??`${item.class} cursor ${item.write_cursor}`),el("p",`Native rank ${item.rank} · ${data.scope.score_family} score ${item.score}`),el("pre",JSON.stringify(item.evidence,null,2),{class:"serving-evidence"}),el("pre",item.payload.text));card.append(button("Copy observed result payload",()=>copyLexical(current,item.payload.text)));vectorOrigins(card,item);results.append(card);}
    if(!data.items.length)results.append(el("p","No native result returned within this query's limits and disclosure policy."));
    results.append(el("pre",JSON.stringify({request:data.request,query_context:data.query_context,scope:data.scope,context:data.context},null,2),{class:"serving-context"}));panel.append(results);
  }
  scope(panel,current.data);
  for(const data of new Set([current.source,current.data]))bind(data,()=>{if(current.source!==data&&current.data!==data)return;scrubDOM(current.workspace);current.workspace=null;if(view===current)queueMicrotask(()=>{if(view===current)render();});});
}

function releaseObservation(data) {
  if (!data?.observation || Object.values(pins).some(slot=>slot.id===data.observation)) return;
  const item = observations.get(data.observation);
  observations.delete(data.observation);
  evict(data, item);
}
function asOfFromRequest(request) {
  if (request?.as_of_write_cursor != null)
    return { mode: "write_cursor", value: String(request.as_of_write_cursor) };
  if (request?.as_of_epoch_ms != null)
    return { mode: "epoch_ms", value: String(request.as_of_epoch_ms) };
  if (request?.as_of?.mode === "write_cursor" && request.as_of.requested_as_of_write_cursor != null)
    return { mode: "write_cursor", value: String(request.as_of.requested_as_of_write_cursor) };
  if (request?.as_of?.mode === "epoch_ms" && request.as_of.requested_as_of_epoch_ms != null)
    return { mode: "epoch_ms", value: String(request.as_of.requested_as_of_epoch_ms) };
  return { mode: "current", value: "" };
}
function validateAsOfInput(mode, raw) {
  const text = String(raw ?? "").trim();
  if (mode === "current") return { ok: true, payload: {} };
  if (mode === "write_cursor") {
    if (!/^(0|[1-9]\d*)$/.test(text))
      return {
        ok: false,
        message:
          "Invalid snapshot write cursor: enter a canonical non-negative 64-bit integer.",
      };
    if (BigInt(text) > 18446744073709551615n)
      return {
        ok: false,
        message: "Invalid snapshot write cursor: exceeds maximum 64-bit unsigned integer.",
      };
    return { ok: true, payload: { as_of_write_cursor: text } };
  }
  if (mode === "epoch_ms") {
    if (!/^(0|-?[1-9]\d*)$/.test(text))
      return {
        ok: false,
        message: "Invalid snapshot commit time (ms): enter a canonical signed 64-bit integer.",
      };
    const v = BigInt(text);
    if (v < -9223372036854775808n || v > 9223372036854775807n)
      return {
        ok: false,
        message: "Invalid snapshot commit time (ms): exceeds 64-bit signed integer range.",
      };
    return { ok: true, payload: { as_of_epoch_ms: text } };
  }
  return { ok: false, message: "Invalid snapshot mode." };
}
function stripAsOf(request) {
  const next = { ...request };
  delete next.as_of_write_cursor;
  delete next.as_of_epoch_ms;
  return next;
}
function renderGraphProperties(parent, label, inlineProps, hydratedRecord) {
  if (inlineProps && typeof inlineProps === "object" && Object.keys(inlineProps).length > 0) {
    parent.append(el("h5", label));
    const dl = el("dl", null, { class: "graph-properties" });
    for (const [k, v] of Object.entries(inlineProps)) {
      dl.append(
        el("dt", k),
        el("dd", v === null ? "null" : v === "" ? "Empty string" : String(v)),
      );
    }
    parent.append(dl);
  } else if (
    hydratedRecord?.properties_status === "available" &&
    hydratedRecord.properties_bytes !== null &&
    hydratedRecord.properties_bytes !== undefined &&
    hydratedRecord.properties !== null
  ) {
    parent.append(
      el("h5", label),
      el(
        "pre",
        hydratedRecord.properties === "" ? "Empty string" : String(hydratedRecord.properties),
        { class: "graph-properties" },
      ),
    );
  } else if (hydratedRecord?.properties_status === "refused") {
    parent.append(el("p", `${label}: refused (erasure-pending)`, { class: "muted" }));
  }
}
async function openGraph(handle) {
  const current=view, focus=document.activeElement, token=++navigation;
  const record=current?.data?.record;
  const seedClass=record?.class??null;
  const expanded=seedClass==="node"?[handle]:[];
  const request={op:"graph",handle,direction:"both",kind:null,depth:3,expanded};
  const data=await api(request,true,()=>token===navigation);
  remember(current,focus);
  view={kind:"graph",handle,seedClass,seedExpanded:expanded.length>0,data,request,asOfDraft:{mode:"current",value:""},graphFeedback:null,selected:data.seed.key,native:null,expansions:new Map(expanded.map(h=>[data.seed.key,h]))};render();
}
async function openInvestigation(handle) {
  const current=view, focus=document.activeElement, token=++navigation;
  const data=await api({op:"investigate",handle},true,()=>token===navigation);
  remember(current,focus);view={kind:"investigation",handle,data};render();
}
function svgElement(tag,attrs={},text) {
  const node=document.createElementNS("http://www.w3.org/2000/svg",tag);
  for(const [name,value] of Object.entries(attrs))node.setAttribute(name,String(value));
  if(text!==undefined)node.textContent=text;
  return node;
}
function graphName(record) { return record.logical_id ?? `${record.class} cursor ${record.write_cursor}`; }
function graphLabel(value,limit) {const chars=Array.from(String(value));return chars.length>limit?chars.slice(0,limit).join("")+"…":String(value);}
async function updateGraph(current,request,owners=current.expansions,seedExpanded=current.seedExpanded) {
  const token=++navigation;
  const data=await api(request,true,()=>token===navigation&&view===current);
  const old=current.data, oldNative=current.native,oldWorkspace=current.workspace,focused=document.activeElement?.textContent;
  const oldSeedKey=old?.seed?.key;
  const nextExpansions=new Map([...owners].filter(([key])=>data.nodes.some(record=>record.key===key)));
  if(((seedExpanded&&current.seedClass==="node")||(oldSeedKey&&owners.has(oldSeedKey)))&&data.seed?.key&&data.nodes.some(record=>record.key===data.seed.key)&&!nextExpansions.has(data.seed.key))
    nextExpansions.set(data.seed.key,data.seed.handle);
  current.seedExpanded=seedExpanded;
  current.expansions=nextExpansions;
  const nextExpanded=[...current.expansions.values()];
  if(seedExpanded&&current.seedClass==="node"&&!nextExpansions.has(data.seed?.key)&&current.handle&&!nextExpanded.includes(current.handle))
    nextExpanded.push(current.handle);
  current.data=data;current.request={...request,expanded:nextExpanded};current.asOfDraft=asOfFromRequest(current.request);current.graphFeedback=null;current.workspace=null;current.native=null;current.returnFocus=null;
  scrubDOM(oldWorkspace);
  if(![...data.nodes,...data.edges].some(v=>v.key===current.selected))current.selected=data.seed.key;
  releaseObservation(old);releaseObservation(oldNative);render();
  const alternate={"Collapse selected node":"Expand selected node","Expand selected node":"Collapse selected node"};
  const buttons=[...current.workspace.querySelectorAll("button")];
  (buttons.find(b=>b.textContent===focused&&!b.disabled)??buttons.find(b=>b.textContent===alternate[focused]&&!b.disabled)??buttons.find(b=>b.dataset.graphKey===current.selected))?.focus();
}
function renderGraph() {
  const current=view,{panel:host}=shell();
  if(current.workspace){current.workspace.firstElementChild.replaceWith(host.firstElementChild);host.replaceWith(current.workspace);return;}
  const panel=host,owner=current.data;current.workspace=panel;
  panel.append(el("h2","Stored directed graph"));
  const seedRecord=[...owner.nodes,...owner.edges].find(r=>r.key===owner.seed?.key)??owner.nodes[0]??owner.edges[0];
  if(seedRecord?.scope!==undefined&&(isMultiScope()||seedRecord.scope!=="")){
    panel.append(el("p",`Scope: ${scopeLabel(seedRecord.scope)}`,{class:"graph-scope muted"}));
  }
  panel.append(el("p","Stored relations · arrows show direction · layout has no metric meaning. Diagram labels are previews; exact identities are available through the list and detail. Expansion and filters create a separate read observation."));
  const form=el("div",null,{class:"graph-filters"});
  const label=el("label","Graph direction"),direction=el("select",null,{"aria-label":"Graph direction"});
  for(const [v,text] of [["both","Both"],["outgoing","Outgoing"],["incoming","Incoming"]])direction.append(el("option",text,{value:v}));
  direction.value=current.request.direction;label.append(direction);
  const [kindLabel,kind]=labelInput("Edge kind filter",current.request.kind??"");
  const initialAsOf=current.asOfDraft??asOfFromRequest(current.request);
  const snapLabel=el("label","Snapshot"),snapshotMode=el("select",null,{"aria-label":"Snapshot"});
  for(const [v,text] of [["current","Current"],["write_cursor","At write cursor"],["epoch_ms","At commit time (ms)"]])snapshotMode.append(el("option",text,{value:v}));
  snapshotMode.value=initialAsOf.mode;snapLabel.append(snapshotMode);
  const [snapValLabel,snapshotInput]=labelInput("Snapshot value",initialAsOf.value??"");
  snapshotInput.setAttribute("aria-label","Snapshot value");
  snapshotInput.disabled=snapshotMode.value==="current";
  const graphFeedback=el("div",null,{role:"status","aria-label":"Graph query feedback",class:"graph-feedback"});
  if(current.graphFeedback)graphFeedback.append(el("p",current.graphFeedback));
  const syncDraft=()=>{
    snapshotInput.disabled=snapshotMode.value==="current";
    current.asOfDraft={mode:snapshotMode.value,value:snapshotInput.value};
    if(snapshotMode.value==="current"){current.graphFeedback=null;graphFeedback.replaceChildren();}
  };
  snapshotMode.addEventListener("change",syncDraft);
  snapshotInput.addEventListener("input",syncDraft);
  form.append(
    label,
    kindLabel,
    snapLabel,
    snapValLabel,
    button("Apply graph filters",()=>{
      const checked=validateAsOfInput(snapshotMode.value,snapshotInput.value);
      if(!checked.ok){
        current.graphFeedback=checked.message;
        graphFeedback.replaceChildren(el("p",checked.message));
        return;
      }
      current.graphFeedback=null;
      graphFeedback.replaceChildren();
      const baseReq=stripAsOf(current.request);
      const expanded=Array.isArray(baseReq.expanded)?[...baseReq.expanded]:[];
      if(current.seedExpanded&&current.seedClass==="node"&&!current.expansions.has(current.data?.seed?.key)&&current.handle&&!expanded.includes(current.handle))
        expanded.push(current.handle);
      return updateGraph(current,{...baseReq,...checked.payload,direction:direction.value,kind:kind.value||null,expanded});
    }),
    button("Refresh this view",()=>updateGraph(current,current.request)),
    graphFeedback,
  );
  panel.append(form);
  pinControls(panel,current,owner);
  const records=[...owner.nodes,...owner.edges];
  const selected=()=>records.find(v=>v.key===current.selected);
  const controls=el("div",null,{class:"actions"});
  const open=button("Open selected record",async()=>{const record=selected();if(record)await openDetail(record.handle);});
  const investigate=button("Investigate selected record",async()=>{const record=selected();if(record)await openInvestigation(record.handle);});
  const expand=button("Expand selected node",()=>{const record=selected();if(!record)return;const owners=new Map(current.expansions);if(owners.size>=100)throw Error("Expansion owner limit reached; collapse first.");owners.set(record.key,record.handle);const seedExpanded=record.key===current.data?.seed?.key?true:current.seedExpanded;return updateGraph(current,{...current.request,expanded:[...owners.values()]},owners,seedExpanded);});
  const collapse=button("Collapse selected node",()=>{const record=selected();if(!record)return;const owners=new Map(current.expansions);owners.delete(record.key);const seedExpanded=record.key===current.data?.seed?.key?false:current.seedExpanded;return updateGraph(current,{...current.request,expanded:[...owners.values()]},owners,seedExpanded);});
  const native=button("Run native graph traversal",async()=>{
    const record=selected();
    const targetHandle=record?.handle??(current.seedClass==="node"?(owner.seed?.handle??current.handle):null);
    if(!targetHandle)return;
    const checked=validateAsOfInput(snapshotMode.value,snapshotInput.value);
    if(!checked.ok){
      current.graphFeedback=checked.message;
      graphFeedback.replaceChildren(el("p",checked.message));
      return;
    }
    current.graphFeedback=null;
    graphFeedback.replaceChildren();
    const token=++navigation;
    const data=await api({op:"graph_serving",handle:targetHandle,direction:current.request.direction,kind:current.request.kind,depth:current.request.depth,...checked.payload},true,()=>token===navigation&&view===current&&current.data===owner);
    const old=current.native,oldWorkspace=current.workspace;current.native=data;current.workspace=null;current.returnFocus=null;scrubDOM(oldWorkspace);releaseObservation(old);render();
    [...current.workspace.querySelectorAll("button")].find(b=>b.textContent==="Run native graph traversal")?.focus();
  });
  controls.append(open,investigate,expand,collapse,native);panel.append(controls);
  const selection=el("p",null,{class:"graph-selection","aria-live":"polite"});panel.append(selection);
  const svg=svgElement("svg",{class:"stored-graph",viewBox:"0 0 1100 760",role:"img","aria-label":"Directed stored graph; use synchronized relationship buttons for keyboard actions"});
  const defs=svgElement("defs"),marker=svgElement("marker",{id:`arrow-${owner.observation}`,viewBox:"0 0 10 10",refX:9,refY:5,markerWidth:7,markerHeight:7,orient:"auto-start-reverse"});marker.append(svgElement("path",{d:"M 0 0 L 10 5 L 0 10 z"}));defs.append(marker);svg.append(defs);
  const positions=new Map(),live=new Set(owner.nodes.map(record=>record.key));
  current.layout=new Map([...(current.layout??[])].filter(([key])=>live.has(key)));
  const used=new Set(current.layout.values());
  for(const record of owner.nodes)if(!current.layout.has(record.key)){let slot=0;while(used.has(slot))slot++;current.layout.set(record.key,slot);used.add(slot);}
  for(const [key,slot] of current.layout)positions.set(key,{x:140+(slot%4)*240,y:90+Math.floor(slot/4)*120});
  const nodeBottom=Math.max(500,...[...positions.values()].map(p=>p.y+100));
  if(owner.nodes.length>12)svg.classList.add("dense");
  const anchors=new Map();
  function endpoint(value) {
    if(value.resolution==="one"&&value.targets.length===1&&positions.has(value.targets[0]))return positions.get(value.targets[0]);
    // This anchor represents an unresolved logical reference, never a selected revision.
    const id=value.key;
    if(!anchors.has(id)){const index=anchors.size;anchors.set(id,{x:140+(index%4)*240,y:nodeBottom+40+Math.floor(index/4)*60,value});}
    return anchors.get(id);
  }
  const list=el("section",null,{"aria-label":"Graph relationships",class:"graph-relationships"});list.append(el("h3","Synchronized relationships"));
  let row=0;
  function select(record,focus=false) {
    current.selected=record.key;for(const label of svg.querySelectorAll("[data-graph-label]"))label.classList.toggle("active",label.dataset.graphLabel===record.key);selection.textContent=`Selected ${record.class}: ${graphName(record)} · ${record.revision_id??"no recorded revision"}`;
    for(const node of panel.querySelectorAll("[data-graph-key]")){const active=node.dataset.graphKey===record.key;node.classList.toggle("selected",active);if(node.tagName==="BUTTON"){node.setAttribute("aria-pressed",String(active));node.tabIndex=active?0:-1;if(active&&focus)node.focus();}}
    open.disabled=investigate.disabled=false;
    expand.disabled=collapse.disabled=native.disabled=record.class!=="node";
    const nodeExpanded=current.expansions.has(record.key);
    if(record.class==="node"){expand.disabled=nodeExpanded;collapse.disabled=!nodeExpanded;}
  }
  function rowButton(record,label) {
    const b=button(label,()=>select(record),{"data-graph-key":record.key,"aria-pressed":"false",tabindex:row++===0?0:-1});list.append(b);return b;
  }
  const groupCounts=new Map(),groupOrdinals=new Map();
  const placements=owner.edges.map(record=>{
    const from=endpoint(record.from),to=endpoint(record.to);
    const ends=[`${from.x}:${from.y}`,`${to.x}:${to.y}`].sort(),group=JSON.stringify(ends);
    groupCounts.set(group,(groupCounts.get(group)??0)+1);
    return {record,from,to,group,self:from===to};
  });
  placements.forEach(({record,from,to,group,self})=>{
    const ordinal=groupOrdinals.get(group)??0,total=groupCounts.get(group);groupOrdinals.set(group,ordinal+1);
    const step=Math.min(25,200/total),loopWidth=90+ordinal*step,loopHeight=100+ordinal*step;
    const first=`${from.x}:${from.y}`<`${to.x}:${to.y}`?from:to,last=first===from?to:from;
    const length=Math.hypot(last.x-first.x,last.y-first.y)||1;
    const offset=50+(ordinal-(total-1)/2)*Math.min(45,480/Math.max(1,total-1));
    const cx=(from.x+to.x)/2-(last.y-first.y)/length*offset,cy=(from.y+to.y)/2+(last.x-first.x)/length*offset;
    const d=self?`M ${from.x} ${from.y-18} C ${from.x+loopWidth} ${from.y-loopHeight},${from.x-loopWidth} ${from.y-loopHeight},${from.x-8} ${from.y-18}`:`M ${from.x} ${from.y} Q ${cx} ${cy},${to.x} ${to.y}`;
    const path=svgElement("path",{d,class:"graph-edge","data-graph-key":record.key,"marker-end":`url(#arrow-${owner.observation})`});path.addEventListener("click",()=>select(record,true));svg.append(path);
    const label=svgElement("text",{x:self?from.x:(from.x+to.x+2*cx)/4,y:self?from.y-loopHeight*0.75:(from.y+to.y+2*cy)/4,class:"graph-kind","data-graph-label":record.key},graphLabel(record.kind??"unknown kind",24));svg.append(label);
    rowButton(record,`Select edge ${graphName(record)}`);
    list.append(el("p",`${record.from.identity} → ${record.to.identity} · ${record.kind??"unknown kind"} · endpoints ${record.from.resolution}/${record.to.resolution} · ${record.revision_id??"no recorded revision"}`));
  });
  for(const anchor of anchors.values()){const g=svgElement("g");g.append(svgElement("rect",{x:anchor.x-45,y:anchor.y-12,width:100,height:30,rx:4,class:"graph-unresolved"}),svgElement("text",{x:anchor.x,y:anchor.y+7,"text-anchor":"middle"},`${graphLabel(anchor.value.identity,14)} (${anchor.value.resolution})`));svg.append(g);}
  for(const record of owner.nodes){
    const pos=positions.get(record.key),g=svgElement("g",{"data-graph-key":record.key,class:"graph-node"});g.append(svgElement("circle",{cx:pos.x,cy:pos.y,r:22}),svgElement("text",{x:pos.x,y:pos.y+38,"text-anchor":"middle"},`${graphLabel(graphName(record),20)} · ${graphLabel(record.revision_id??record.write_cursor,14)}`));g.addEventListener("click",()=>select(record,true));svg.append(g);
    rowButton(record,`Select node ${graphName(record)} cursor ${record.write_cursor} ${record.revision_id??"no recorded revision"}`);
  }
  list.addEventListener("keydown",event=>{
    if(!["ArrowDown","ArrowUp","Home","End"].includes(event.key))return;
    const rows=[...list.querySelectorAll("button")],at=rows.indexOf(document.activeElement);if(at<0)return;
    event.preventDefault();const index=event.key==="Home"?0:event.key==="End"?rows.length-1:(at+(event.key==="ArrowDown"?1:-1)+rows.length)%rows.length;rows[index].focus();
  });
  const viewport=el("div",null,{class:"graph-viewport",tabindex:"0","aria-label":"Scrollable graph viewport"}),camera=el("div",null,{class:"actions","aria-label":"Graph viewport controls"});
  viewport.append(svg);panel.append(camera);
  const asOfSummary=owner.scope?.as_of&&owner.scope.as_of.mode!=="current"?` Snapshot: ${owner.scope.as_of.mode} (resolved cursor ${owner.scope.as_of.resolved_as_of_write_cursor}).`:"";
  panel.append(viewport,el("p",`${owner.status}: ${owner.nodes.length} retained nodes, ${owner.edges.length} edges; ${owner.scope.examined_links} examined incident links.${asOfSummary} Endpoint resolution: ${owner.scope.endpoint_resolution_complete?"covered":"unknown/partial"}. Missing-result cause: undetermined.`,{class:"graph-summary"}),list,el("pre",JSON.stringify(owner.context,null,2),{class:"graph-context"}));
  const bounds=svg.getBBox(),width=Math.max(1100,bounds.width+80),height=Math.max(760,bounds.height+80);
  svg.setAttribute("viewBox",`${bounds.x+(bounds.width-width)/2} ${bounds.y+(bounds.height-height)/2} ${width} ${height}`);
  let zoom=current.zoom??1;
  const size=()=>{svg.style.width=`${width*zoom}px`;svg.style.height=`${height*zoom}px`;current.zoom=zoom;};
  camera.append(button("Zoom in graph",()=>{zoom=Math.min(3,zoom*1.25);size();}),button("Zoom out graph",()=>{zoom=Math.max(.35,zoom/1.25);size();}),button("Fit graph width",()=>{zoom=Math.max(.35,Math.min(1,viewport.clientWidth/width));size();}),button("Readable graph scale",()=>{zoom=1;size();}),button("Show selected in graph",()=>{const pos=positions.get(current.selected);if(pos){viewport.scrollLeft=Math.max(0,pos.x*zoom-viewport.clientWidth/2);viewport.scrollTop=Math.max(0,pos.y*zoom-viewport.clientHeight/2);}viewport.scrollIntoView({block:"nearest"});viewport.focus();}));size();
  scope(panel,owner);
  if(records.length)select(selected()??records[0]);
  else {
    open.disabled=investigate.disabled=expand.disabled=collapse.disabled=true;
    native.disabled=current.seedClass!=="node";
    selection.textContent="No retained records in this snapshot.";
  }
  if(current.native&&!current.native.evicted){
    const data=current.native,section=el("section",null,{class:"graph-native","aria-label":"Actual frozen graph evidence"});section.append(el("h3","Actual frozen graph evidence"),el("p","Actual selected target and terminal edge · no rank. Later origin hydration is a separate observation. Missing-result cause: undetermined."));
    if(!data.items.length)section.append(el("p",data.query_context?.seed_absent_at_as_of?"No native graph results: seed absent at requested snapshot.":"No permitted native graph targets in this observation."));
    for(const item of data.items){
      section.append(el("h4","Original frozen target payload"),el("pre",item.payload.text,{class:"graph-native-payload"}),el("p",`${item.payload.scope} · ${item.payload.total_bytes} bytes · window complete ${item.payload.complete}`),button("Copy observed graph payload window",()=>copyLexical(current,item.payload.text)));
      renderGraphProperties(section,"Target properties",item.payload.properties,item.target);
      renderGraphProperties(section,"Terminal edge properties",item.payload.terminal_edge_properties,item.terminal_edge);
      for(const [label,record] of [["native target",item.target],["native terminal edge",item.terminal_edge]])section.append(button(`Open ${label} ${record.revision_id??graphName(record)}`,()=>openDetail(record.handle)));
      metadata(section,item.evidence);
    }
    pinControls(section,current,data,"native graph ");
    section.append(el("h4","Original request and frozen observation"),el("pre",JSON.stringify({request:data.request,...data.query_context},null,2),{class:"native-graph-context"}));scope(section,data);panel.append(section);
    bind(data,()=>{if(current.native!==data)return;section.replaceChildren(el("p","Native evidence evicted; run explicitly again."));current.native=null;});
  }
  bind(owner,()=>{if(current.data!==owner)return;scrubDOM(current.workspace);current.workspace=null;current.native=null;if(view===current)queueMicrotask(()=>{if(view===current)render();});});
}
function renderInvestigation() {
  const current=view,{panel:host}=shell();
  if(current.workspace){current.workspace.firstElementChild.replaceWith(host.firstElementChild);host.replaceWith(current.workspace);return;}
  const panel=host,data=current.data;current.workspace=panel;
  panel.append(el("h2","Provenance and retained history"),el("p","Recorded source attribution, registered dependency and retained revisions keep distinct meanings. Each navigation is a separate read; missing-result cause remains undetermined."),button("Refresh this view",refreshView),el("h3","Observed record"));metadata(panel,data.record);
  const revisions=el("section",null,{"aria-label":"Retained canonical revisions"});revisions.append(el("h3","Retained canonical revisions"),el("p",`${data.scope.history_complete?"Covered":"Partial/unknown"} · ordered by internal write cursor; no inferred causal successor.`));
  for(const record of data.revisions)revisions.append(button(`Open retained revision ${record.revision_id??`${record.class} cursor ${record.write_cursor}`}`,()=>openDetail(record.handle)),el("p",`${record.class} · ${graphName(record)} · superseded_at ${record.superseded_at??"NULL"}`));
  const links=el("section",null,{"aria-label":"Recorded investigation links"});links.append(el("h3","Recorded investigation links"));
  for(const link of data.links){links.append(el("h4",link.kind.replaceAll("_"," ")),el("p",link.resolution));if(link.target)links.append(button(`Open ${link.kind.replaceAll("_"," ")} ${link.target.revision_id??graphName(link.target)}`,()=>openDetail(link.target.handle)));if(link.metadata)metadata(links,link.metadata);}
  panel.append(revisions,links,el("pre",JSON.stringify(data.context,null,2),{class:"investigation-context"}));scope(panel,data);
  bind(data,()=>{if(current.data!==data)return;scrubDOM(current.workspace);current.workspace=null;if(view===current)queueMicrotask(()=>{if(view===current)render();});});
}

function collectComparisonHandles(item,content,metadataHandles) {
  const data=item.data;
  if(item.request.handle)content.add(item.request.handle);
  for(const handle of [data.scope?.origin?.handle,data.seed?.origin?.handle])if(handle)content.add(handle);
  for(const entry of entries(data,item.request))for(const origin of entry.origins)
    if(origin.handle)(origin.metadataOnly?metadataHandles:content).add(origin.handle);
}
function pinDescriptor(current,data) {
  const obsReq=observations.get(data.observation)?.request??current.request;
  return {kind:data.scope?.route==="actual_serving_graph_evidence"?"graph_evidence":current.kind,
    handle:current.handle??null,seedClass:current.seedClass??null,seedExpanded:current.seedExpanded??false,source:current.source?.observation??null,declarations:current.declarations?.observation??null,
    asOf:asOfFromRequest(obsReq),
    selected:current.selected??null,expanded:current.expansions?[...current.expansions]:null};
}
function pinControls(parent,current,data,prefix="") {
  if(!data?.observation||data.evicted)return;
  const controls=el("div",null,{class:"actions","aria-label":`${prefix}observation pins`});
  for(const side of ["A","B"])controls.append(button(`${pins[side].id?"Replace":"Pin"} ${prefix}${side}`,async()=>{
    const slot=pins[side],version=++slot.version,at=cacheVersion,nav=navigation;
    const accept=()=>view===current&&nav===navigation&&version===slot.version&&at===cacheVersion&&!data.evicted&&(current.data===data||current.native===data);
    if(!(await validate(current,accept))||!accept()||!observations.has(data.observation))return;
    const old=slot.id;
    slot.id=data.observation;slot.descriptor=pinDescriptor(current,data);delete slot.state;
    clearComparison();
    if(cacheBytes()>cacheCap){slot.id=null;slot.descriptor=null;slot.state="budget refused";slot.version++;say(`${side} pin refused: observation budget exhausted.`);}
    else say(`${side} pinned.`);
    releasePinOnly(old);render();
  },{"data-pin-side":side,"data-pin-prefix":prefix}));
  parent.append(controls);
}
function releasePinOnly(id) {
  if(!id||Object.values(pins).some(slot=>slot.id===id))return;
  const data=observations.get(id)?.data;
  if(data&&![view,...history].some(saved=>saved?.data===data||saved?.source===data||saved?.native===data||saved?.associations===data))releaseObservation(data);
}
function unpin(side) {
  const slot=pins[side],old=slot.id;
  slot.version++;slot.id=null;slot.descriptor=null;delete slot.state;
  clearComparison();releasePinOnly(old);render();
  say(`${side} unpinned.`);
  [...root.querySelectorAll("button")].find(b=>b.textContent==="Compare observations")?.focus();
}
function pinToken() {return [pins.A.version,pins.B.version,cacheVersion,navigation].join(":");}
async function comparisonAction(action) {
  const current=view,token=pinToken(),accept=()=>view===current&&token===pinToken();
  if(await validate(current,accept)&&accept())await action();
}
async function openComparison() {
  const previous=view,focus=document.activeElement,token=++navigation;
  const accept=()=>navigation===token&&view===previous;
  if(!(await validate(previous,accept))||!accept())return;
  remember(previous,focus);view={kind:"comparison",data:{},mode:"union",start:0};render();
}
function sideContext(item) {
  const data=item.data;
  return {request:data.request??item.request,submitted_request:item.request,seed:data.seed??null,status:data.status,
    scope:data.scope,admission:data.admission??null,metadata:data.metadata??null,original_query_context:data.query_context??null,later_policy_read_context:data.context,
    population:data.scope?.population??"unknown; inspect route-specific scope facts",
    meaning:"Separate actual observation. No shared snapshot or inferred omission cause."};
}
function renderComparison() {
  clearComparison();
  const current=view,{panel}=shell();current.workspace=panel;panel.classList.add("comparison-workspace");
  panel.append(el("h2","Compare observations"),el("p","A missing side means not returned in this bounded observation. It does not establish database absence, rank zero or an omission cause. Each method retains its own values; observations keep separate contexts."));
  const sides={};
  for(const side of ["A","B"]) {
    const slot=pins[side],item=observations.get(slot.id),section=el("section",null,{class:`comparison-side-${side}`,"aria-label":`Observation ${side}`});
    section.append(el("h3",`Observation ${side}`));
    if(item){sides[side]=item;section.append(button(`Open ${side} original observation`,()=>comparisonAction(()=>openPinned(side))),button(`Unpin ${side}`,()=>unpin(side)));}
    else section.append(el("p",slot.state?`Observation ${slot.state}; select and pin a fresh actual observation explicitly.`:"Observation not pinned."));
    panel.append(section);
  }
  // Charge transient adapters, key indexes, descriptor text, DOM and captured controls
  // before constructing them. Pinning has no additional observation/byte allowance.
  const owners=[...new Set(Object.values(sides))];
  comparisonBytes=65536+owners.reduce((n,item)=>n+item.bytes*4,0);
  if(cacheBytes()>cacheCap){comparisonBytes=0;panel.append(el("p","Comparison refused: shared observation byte budget exhausted. Unpin or release observations and try explicitly.",{role:"alert"}));return;}
  for(const side of Object.keys(sides)){const parent=panel.querySelector(`.comparison-side-${side}`);observationSummary(parent,sides[side].data,sides[side].request);exactEvidence(parent,"Exact original observation",el("pre",JSON.stringify(sideContext(sides[side]),null,2)));}
  const label=el("label","Comparison membership"),mode=el("select",null,{"aria-label":"Comparison membership"});
  for(const value of ["union","intersection"])mode.append(el("option",value,{value}));mode.value=current.mode;
  mode.addEventListener("change",()=>run(()=>comparisonAction(()=>{current.mode=mode.value;current.start=0;render();})));label.append(mode);panel.append(label);
  const left=entries(sides.A?.data,sides.A?.request),right=entries(sides.B?.data,sides.B?.request);
  const page=comparisonPage(left,right,{mode:current.mode,start:current.start});
  panel.append(el("p","Membership compares typed native result entries, not canonical origins. Distinct entries can refer to the same exact record revision.",{class:"membership-unit"}));
  const matches=canonicalCorrespondence(left,right),correspondences=el("section",null,{class:"canonical-correspondences","aria-label":"Verified canonical correspondences"});
  correspondences.append(el("h3","Verified canonical correspondences"),el("p",`${matches.size} shared exact canonical revisions; native entries, ranks and metrics remain separate.`));
  for(const group of matches.values()){
    const origin=group.A[0].origins[0],record=origin.record,name=record.logical_id??record.revision_id;
    correspondences.append(el("p",`${name} · ${record.class} cursor ${record.write_cursor} · revision ${record.revision_id} · A: ${group.A.length} native entries; B: ${group.B.length} native entries`),button(`Open shared exact origin ${name}`,()=>comparisonAction(()=>openDetail(origin.handle))));
  }
  panel.append(correspondences);
  panel.append(el("p",`${page.total} typed entries in bounded ${current.mode}; showing ${page.rows.length?current.start+1:0}–${current.start+page.rows.length}.`,{class:"comparison-count"}));
  for(const row of page.rows) {
    const article=el("article",null,{class:"comparison-row"});article.append(el("h3",(row.A??row.B).label),el("p",row.A&&row.B?"Returned in A and B":row.A?"Returned in A only":"Returned in B only"));
    for(const side of ["A","B"]) {
      const entry=row[side],section=el("section",null,{"aria-label":`${side} entry`});section.append(el("h4",side));
      if(!entry)section.append(el("p","This typed result entry was not returned in this bounded observation; inspect verified canonical correspondences separately."));
      else {
        section.append(el("pre",JSON.stringify(entry.identity,null,2),{class:"comparison-identity"}),el("p",entry.rank===null?"Rank: not applicable or unavailable":`Original rank: ${entry.rank}`),el("p",`${entry.metric} · ${entry.direction} · value: ${entry.value??"not applicable or unavailable"}`));
        if(entry.item.representative_branch)section.append(el("p",`Representative arm: ${entry.item.representative_branch}; does not describe every contributor.`));
        if(entry.item.payload)section.append(el("pre",entry.item.payload.text,{class:"comparison-payload"}),el("p",`Original payload scope ${entry.item.payload.scope??"unknown"}; complete ${entry.item.payload.complete??"unknown"}; bytes ${entry.item.payload.total_bytes??"unknown"}`));
        section.append(el("p",`${entry.origins.length} recorded exact origin choices; origin correlation does not merge result entries.`));
        for(const origin of entry.origins.slice(0,5))if(origin.handle) {
          const name=origin.record.logical_id??origin.record.revision_id??origin.record.write_cursor;
          const open=button(`Open ${side} origin ${name}`,()=>comparisonAction(()=>openDetail(origin.handle)));
          open.disabled=!!origin.metadataOnly;section.append(open);
          if(origin.metadataOnly)section.append(el("p","Payload withheld; this observation contains metadata only."));
        }
        if(entry.origins.length>5)section.append(el("p","Additional exact origin choices are available in the original observation."));
      }
      article.append(section);
    }
    panel.append(article);
  }
  const controls=el("div",null,{class:"actions"});
  if(current.start)controls.append(button("Previous comparison page",()=>comparisonAction(()=>{current.start=Math.max(0,current.start-50);render();})));
  if(current.start+page.rows.length<page.total)controls.append(button("Next comparison page",()=>comparisonAction(()=>{current.start+=50;render();})));
  panel.append(controls);
}
async function openPinned(side) {
  const slot=pins[side],item=observations.get(slot.id);if(!item)return;
  const descriptor=slot.descriptor,request=item.request,previous=view,focus=document.activeElement;
  navigation++;remember(previous,focus);
  const restoredRequest=structuredClone(item.data.request??request);
  view={...descriptor,data:item.data,source:observations.get(descriptor.source)?.data??{evicted:true},declarations:observations.get(descriptor.declarations)?.data??null,
    request:restoredRequest,asOfDraft:descriptor.asOf?structuredClone(descriptor.asOf):asOfFromRequest(request),graphFeedback:null,
    className:request.class,filter:request.filter,after:request.after,sample:request.op==="sample",
    lastRequest:request.op==="serving"||request.op==="neighborhood"?request:null,
    nativeQuery:request.query??"",expansions:new Map(descriptor.expanded??[]),native:null};
  if(!["lexical","vector"].includes(view.kind))delete view.source;
  render();
}
function renderFrozenGraph() {
  const current=view,{panel}=shell(),data=current.data;current.workspace=panel;
  panel.append(el("h2","Actual frozen graph evidence"),el("pre",JSON.stringify(sideContext(observations.get(data.observation)),null,2),{class:"native-graph-context"}));
  for(const entry of entries(data,{op:"graph_serving"})) {
    const section=el("section");section.append(el("h3",entry.label),el("pre",entry.item.payload.text),el("pre",JSON.stringify(entry.item.evidence,null,2)));
    renderGraphProperties(section,"Target properties",entry.item.payload.properties,entry.item.target);
    renderGraphProperties(section,"Terminal edge properties",entry.item.payload.terminal_edge_properties,entry.item.terminal_edge);
    for(const origin of entry.origins)if(origin.handle)section.append(button(`Open native origin ${origin.record.revision_id??origin.record.write_cursor}`,()=>openDetail(origin.handle)));
    panel.append(section);
  }
  pinControls(panel,current,data,"native graph ");
  bind(data,()=>{scrubDOM(current.workspace);current.workspace=null;});
}
