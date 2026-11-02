use rusqlite::{params, Connection};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Independent lexical statistical families, matching production separation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Node body text.
    Node,
    /// Edge body text.
    Edge,
    /// Declared scalar node properties, with one cross-name corpus.
    Property,
}

impl Family {
    fn label(self) -> &'static str {
        match self {
            Self::Node => "nodes",
            Self::Edge => "edges",
            Self::Property => "properties",
        }
    }
    fn all() -> [Self; 3] {
        [Self::Node, Self::Edge, Self::Property]
    }
}

/// Explicit caller intent; it is never inferred from replacement order.
#[derive(Clone, Copy, Debug)]
pub enum Intent {
    /// The first assertion of a claim.
    Initial,
    /// A representation revision preserving the asserted validity.
    Revision,
    /// A domain change effective over the caller's supplied valid interval.
    WorldChange,
    /// A correction of the accepted account of that valid interval.
    Correction,
    /// Withdrawal without resurrecting an older assertion.
    Retraction,
}

impl Intent {
    fn label(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Revision => "revision",
            Self::WorldChange => "world_change",
            Self::Correction => "correction",
            Self::Retraction => "retraction",
        }
    }
}

/// One explicitly timed assertion supplied by the experiment workload.
#[derive(Clone, Debug)]
pub struct Change {
    claim: String,
    body: String,
    family: Family,
    properties: BTreeMap<String, String>,
    known: i64,
    valid_from: i64,
    valid_to: Option<i64>,
    intent: Intent,
    source: String,
    endpoints: Option<(String, String)>,
}

/// A coherent temporal snapshot or an explicitly non-snapshot archive.
#[derive(Clone, Copy, Debug)]
pub enum View {
    /// Valid at `valid`, using latest knowledge or the supplied knowledge tick.
    Snapshot { valid: i64, known: Option<i64> },
    /// All retained revisions; optionally include explicit retraction records.
    Archive { include_retracted: bool },
}

/// Alternative mechanisms compared on the same canonical snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// Maintained current FTS with certified scope.
    Partitioned,
    /// All-version postings with selected-corpus BM25 statistics.
    Custom,
    /// Private selected-corpus FTS using native BM25.
    Materialized,
}

/// A ranked revision with a stable claim identity and exact canonical version.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    claim: String,
    version: i64,
    body: String,
    score: f64,
    provenance: Provenance,
}

/// Intrinsic assertion metadata, including both clocks and withdrawal status.
#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    source: String,
    intent: String,
    valid_from: i64,
    valid_to: Option<i64>,
    known_from: i64,
    known_to: Option<i64>,
    accepted: bool,
}

#[derive(Clone, Debug)]
struct Document {
    id: i64,
    claim: String,
    version: i64,
    body: String,
    name: String,
    text: String,
    provenance: Provenance,
}

/// Disposable SQLite prototype, isolated from production database formats.
pub struct Store {
    conn: Connection,
}

fn reject(message: &str) -> Box<dyn std::error::Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

impl Store {
    /// Open/create the experiment format; `now` initializes its materialized view.
    pub fn open(path: &Path, now: i64) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA temp_store=MEMORY;
            CREATE TABLE IF NOT EXISTS config(now INTEGER NOT NULL, known INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS sources(id TEXT PRIMARY KEY, allowed INTEGER NOT NULL,
                active INTEGER NOT NULL, valid_from INTEGER NOT NULL, valid_to INTEGER);
            INSERT OR IGNORE INTO sources VALUES('source',1,1,0,NULL);
            CREATE TABLE IF NOT EXISTS claim_admission(claim TEXT PRIMARY KEY,active INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS assertions(
                id INTEGER PRIMARY KEY AUTOINCREMENT, claim TEXT NOT NULL, body TEXT NOT NULL,
                family TEXT NOT NULL, properties TEXT NOT NULL, source TEXT NOT NULL REFERENCES sources(id),
                from_id TEXT, to_id TEXT, vf INTEGER NOT NULL, vt INTEGER,
                kf INTEGER NOT NULL, kt INTEGER,
                accepted INTEGER NOT NULL DEFAULT 1, intent TEXT NOT NULL,
                CHECK(vt IS NULL OR vf<vt), CHECK(kt IS NULL OR kf<kt));
            CREATE INDEX IF NOT EXISTS claim_head ON assertions(claim,kt);
            CREATE TABLE IF NOT EXISTS projections(id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL REFERENCES assertions(id), family TEXT NOT NULL,
                name TEXT NOT NULL, text TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS projection_owner ON projections(version,family);")?;
        if conn.query_row("SELECT count(*) FROM config", [], |r| r.get::<_, i64>(0))? == 0 {
            conn.execute("INSERT INTO config VALUES(?1,0)", [now])?;
        }
        for family in Family::all() {
            let label = family.label();
            conn.execute_batch(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS all_{label} USING fts5(text, tokenize='porter unicode61 remove_diacritics 2');
                 CREATE VIRTUAL TABLE IF NOT EXISTS current_{label} USING fts5(text, tokenize='porter unicode61 remove_diacritics 2');
                 CREATE VIRTUAL TABLE IF NOT EXISTS vocab_{label} USING fts5vocab(all_{label},'instance');"
            ))?;
        }
        Ok(Self { conn })
    }

    /// Atomically append a whole-interval revision and maintain both projections.
    pub fn put(&mut self, change: Change) -> Result<()> {
        self.put_inner(change, false)
    }

    fn put_inner(&mut self, change: Change, fault: bool) -> Result<()> {
        let tx = self.conn.transaction()?;
        validate_change(&tx, &change)?;
        let originals = current_claim(&tx, &change.claim)?;
        remove_current_claim(&tx, &change.claim)?;
        for (id, old, _) in originals {
            let overlaps = old.valid_to.is_none_or(|end| change.valid_from < end)
                && change.valid_to.is_none_or(|end| old.valid_from < end);
            if !overlaps {
                continue;
            }
            tx.execute("UPDATE assertions SET kt=?1 WHERE id=?2", params![change.known, id])?;
            if old.valid_from < change.valid_from {
                let mut left = old.clone();
                left.valid_to = Some(change.valid_from);
                left.known = change.known;
                insert_assertion(&tx, &left)?;
            }
            if let Some(end) = change.valid_to {
                if old.valid_to.is_none_or(|old_end| end < old_end) {
                    let mut right = old.clone();
                    right.valid_from = end;
                    right.known = change.known;
                    insert_assertion(&tx, &right)?;
                }
            }
        }
        insert_assertion(&tx, &change)?;
        if fault {
            return Err(reject("injected failure after canonical/history insertion"));
        }
        refresh_current_claim(&tx, &change.claim)?;
        tx.execute("UPDATE config SET known=?1", [change.known])?;
        tx.commit()?;
        Ok(())
    }

    /// Inject a failure after canonical/history insertion to qualify rollback.
    pub fn put_fault(&mut self, change: Change) -> Result<()> {
        self.put_inner(change, true)
    }

    /// Set current source authority atomically with affected current projections.
    pub fn configure_source(
        &mut self,
        id: &str,
        allowed: bool,
        active: bool,
        valid_from: i64,
        valid_to: Option<i64>,
    ) -> Result<()> {
        if id.is_empty() || valid_to.is_some_and(|end| end <= valid_from) {
            return Err(reject("invalid source interval"));
        }
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO sources VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET
            allowed=excluded.allowed,active=excluded.active,valid_from=excluded.valid_from,valid_to=excluded.valid_to",
            params![id,allowed,active,valid_from,valid_to])?;
        let claims = source_claims(&tx, id)?;
        for claim in claims {
            remove_current_claim(&tx, &claim)?;
            refresh_current_claim(&tx, &claim)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Change current assertion admission atomically with its current projections.
    pub fn set_active(&mut self, claim: &str, active: bool) -> Result<()> {
        let tx = self.conn.transaction()?;
        remove_current_claim(&tx, claim)?;
        tx.execute("UPDATE claim_admission SET active=?1 WHERE claim=?2", params![active, claim])?;
        refresh_current_claim(&tx, claim)?;
        tx.commit()?;
        Ok(())
    }

    /// Remove every version and projection for a source; physical erasure is unqualified.
    pub fn erase_source(&mut self, id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        for family in Family::all() {
            for prefix in ["all", "current"] {
                tx.execute(&format!("DELETE FROM {prefix}_{} WHERE rowid IN
                    (SELECT p.id FROM projections p JOIN assertions a ON a.id=p.version WHERE a.source=?1)",family.label()),[id])?;
            }
        }
        tx.execute(
            "DELETE FROM projections WHERE version IN (SELECT id FROM assertions WHERE source=?1)",
            [id],
        )?;
        tx.execute("DELETE FROM claim_admission WHERE claim IN (SELECT claim FROM assertions WHERE source=?1)",[id])?;
        tx.execute("DELETE FROM assertions WHERE source=?1", [id])?;
        tx.execute("UPDATE sources SET allowed=0,active=0 WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Reconstruct both index families from retained canonical versions and authority.
    pub fn rebuild(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for family in Family::all() {
            tx.execute(&format!("DELETE FROM all_{}", family.label()), [])?;
            tx.execute(&format!("DELETE FROM current_{}", family.label()), [])?;
        }
        tx.execute("DELETE FROM projections", [])?;
        let claims = {
            let mut stmt = tx.prepare("SELECT DISTINCT claim FROM assertions ORDER BY claim")?;
            let claims = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            claims
        };
        for claim in claims {
            for (version, c, _) in claim_versions(&tx, &claim, false)? {
                insert_projection(&tx, version, c.family, "", &c.body)?;
                if c.family == Family::Node {
                    for (name, text) in &c.properties {
                        insert_projection(&tx, version, Family::Property, name, text)?;
                    }
                }
            }
            refresh_current_claim(&tx, &claim)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Expand one V/K snapshot; return unique target ID, selected body and hop count.
    pub fn graph(
        &self,
        seed: &str,
        view: View,
        depth: usize,
        limit: usize,
    ) -> Result<Vec<(String, String, usize)>> {
        if matches!(view, View::Archive { .. }) {
            return Err(reject("archive union is not a graph snapshot"));
        }
        let tx = self.conn.unchecked_transaction()?;
        let nodes = documents_view(&tx, Family::Node, view)?
            .into_iter()
            .map(|d| (d.claim.clone(), d))
            .collect::<BTreeMap<_, _>>();
        if !nodes.contains_key(seed) || limit == 0 {
            return Ok(Vec::new());
        }
        let edges = documents_view(&tx, Family::Edge, view)?;
        let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for edge in edges {
            let endpoints = tx.query_row(
                "SELECT from_id,to_id FROM assertions WHERE id=?1",
                [edge.version],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
            )?;
            if let (Some(from), Some(to)) = endpoints {
                if nodes.contains_key(&from) && nodes.contains_key(&to) {
                    adjacency.entry(from).or_default().insert(to);
                }
            }
        }
        let mut visited = BTreeSet::from([seed.to_string()]);
        let mut queue = VecDeque::from([(seed.to_string(), 0usize)]);
        let mut targets = Vec::new();
        while let Some((node, hop)) = queue.pop_front() {
            if hop >= depth {
                continue;
            }
            if let Some(neighbors) = adjacency.get(&node) {
                for target in neighbors {
                    if visited.insert(target.clone()) {
                        targets.push((target.clone(), nodes[target].body.clone(), hop + 1));
                        if targets.len() == limit {
                            return Ok(targets);
                        }
                        queue.push_back((target.clone(), hop + 1));
                    }
                }
            }
        }
        tx.commit()?;
        Ok(targets)
    }

    /// Select unique admitted endpoint IDs from ranked edge text before its cap.
    pub fn lexical_seeds(&self, query: &str, view: View, limit: usize) -> Result<Vec<String>> {
        if matches!(view, View::Archive { .. }) {
            return Err(reject("archive union has no graph seeds"));
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        let tx = self.conn.unchecked_transaction()?;
        let nodes = documents_view(&tx, Family::Node, view)?
            .into_iter()
            .map(|d| d.claim)
            .collect::<BTreeSet<_>>();
        let ranked =
            search_on(&tx, query, Family::Edge, None, usize::MAX, view, Backend::Partitioned)?;
        let mut seen = BTreeSet::new();
        let mut seeds = Vec::new();
        for hit in ranked {
            let (from, to) = tx.query_row(
                "SELECT from_id,to_id FROM assertions WHERE id=?1",
                [hit.version],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
            )?;
            if !from.as_ref().is_some_and(|id| nodes.contains(id))
                || !to.as_ref().is_some_and(|id| nodes.contains(id))
            {
                continue;
            }
            for id in [from, to].into_iter().flatten() {
                if nodes.contains(&id) && seen.insert(id.clone()) {
                    seeds.push(id);
                    if seeds.len() == limit {
                        return Ok(seeds);
                    }
                }
            }
        }
        tx.commit()?;
        Ok(seeds)
    }

    /// Compare one backend under an explicitly selected temporal view.
    #[allow(clippy::too_many_arguments)]
    pub fn search_backend(
        &self,
        query: &str,
        family: Family,
        name: Option<&str>,
        limit: usize,
        view: View,
        backend: Backend,
    ) -> Result<Vec<Hit>> {
        let tx = self.conn.unchecked_transaction()?;
        let hits = search_on(&tx, query, family, name, limit, view, backend)?;
        tx.commit()?;
        Ok(hits)
    }

    /// Search a coherent temporal selection or explicit archive under one read snapshot.
    pub fn search_view(
        &self,
        query: &str,
        family: Family,
        name: Option<&str>,
        limit: usize,
        view: View,
    ) -> Result<Vec<Hit>> {
        self.search_backend(query, family, name, limit, view, Backend::Partitioned)
    }

    /// Search the certified current projection at its initial validity instant.
    pub fn search(
        &self,
        query: &str,
        family: Family,
        name: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Hit>> {
        let tx = self.conn.unchecked_transaction()?;
        let now = tx.query_row("SELECT now FROM config", [], |r| r.get::<_, i64>(0))?;
        let hits = search_on(
            &tx,
            query,
            family,
            name,
            limit,
            View::Snapshot { valid: now, known: None },
            Backend::Partitioned,
        )?;
        tx.commit()?;
        Ok(hits)
    }
}

fn source_claims(conn: &Connection, source: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT claim FROM assertions WHERE source=?1")?;
    let result = stmt
        .query_map([source], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(result)
}

fn current_claim(conn: &Connection, claim: &str) -> Result<Vec<(i64, Change, bool)>> {
    claim_versions(conn, claim, true)
}

fn claim_versions(
    conn: &Connection,
    claim: &str,
    head_only: bool,
) -> Result<Vec<(i64, Change, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT id,body,family,properties,source,from_id,to_id,vf,vt,kf,intent,(SELECT active FROM claim_admission ca WHERE ca.claim=assertions.claim)
        FROM assertions WHERE claim=?1 AND (?2=0 OR kt IS NULL) ORDER BY vf,id",
    )?;
    let mut result = Vec::new();
    let mut rows = stmt.query(params![claim, head_only])?;
    while let Some(r) = rows.next()? {
        let family = match r.get::<_, String>(2)?.as_str() {
            "nodes" => Family::Node,
            "edges" => Family::Edge,
            _ => return Err(reject("invalid canonical family")),
        };
        let intent = match r.get::<_, String>(10)?.as_str() {
            "initial" => Intent::Initial,
            "revision" => Intent::Revision,
            "world_change" => Intent::WorldChange,
            "correction" => Intent::Correction,
            "retraction" => Intent::Retraction,
            _ => return Err(reject("invalid intent")),
        };
        let from = r.get::<_, Option<String>>(5)?;
        let to = r.get::<_, Option<String>>(6)?;
        result.push((
            r.get(0)?,
            Change {
                claim: claim.into(),
                body: r.get(1)?,
                family,
                properties: serde_json::from_str(&r.get::<_, String>(3)?)?,
                source: r.get(4)?,
                endpoints: from.zip(to),
                valid_from: r.get(7)?,
                valid_to: r.get(8)?,
                known: r.get(9)?,
                intent,
            },
            r.get(11)?,
        ));
    }
    Ok(result)
}

fn read_provenance(r: &rusqlite::Row<'_>) -> rusqlite::Result<Provenance> {
    Ok(Provenance {
        source: r.get(6)?,
        intent: r.get(7)?,
        valid_from: r.get(8)?,
        valid_to: r.get(9)?,
        known_from: r.get(10)?,
        known_to: r.get(11)?,
        accepted: r.get(12)?,
    })
}

fn documents_view(conn: &Connection, family: Family, view: View) -> Result<Vec<Document>> {
    match view {
        View::Snapshot { valid, known } => documents(conn, family, valid, known),
        View::Archive { include_retracted } => {
            let mut stmt=conn.prepare("SELECT p.id,a.claim,a.id,a.body,p.name,p.text,a.source,a.intent,a.vf,a.vt,a.kf,a.kt,a.accepted
                FROM projections p JOIN assertions a ON a.id=p.version JOIN sources s ON s.id=a.source
                WHERE p.family=?1 AND EXISTS(SELECT 1 FROM claim_admission ca WHERE ca.claim=a.claim AND ca.active=1)
                AND (a.accepted=1 OR ?2=1) AND s.allowed=1 AND s.active=1
                AND s.valid_from<=(SELECT now FROM config) AND (s.valid_to IS NULL OR (SELECT now FROM config)<s.valid_to)
                ORDER BY a.claim,a.id,p.name")?;
            let rows = stmt
                .query_map(params![family.label(), include_retracted], |r| {
                    Ok(Document {
                        id: r.get(0)?,
                        claim: r.get(1)?,
                        version: r.get(2)?,
                        body: r.get(3)?,
                        name: r.get(4)?,
                        text: r.get(5)?,
                        provenance: read_provenance(r)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        }
    }
}

fn validate_change(conn: &Connection, c: &Change) -> Result<()> {
    let known: i64 = conn.query_row("SELECT known FROM config", [], |r| r.get(0))?;
    if c.claim.is_empty()
        || c.known <= known
        || c.valid_to.is_some_and(|end| end <= c.valid_from)
        || c.family == Family::Property
    {
        return Err(reject("invalid claim, interval, family or nonmonotonic knowledge tick"));
    }
    let mut stmt =
        conn.prepare("SELECT family,source,from_id,to_id FROM assertions WHERE claim=?1 LIMIT 1")?;
    let mut rows = stmt.query([&c.claim])?;
    if let Some(row) = rows.next()? {
        let endpoints = (row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?);
        let proposed =
            (c.endpoints.as_ref().map(|e| e.0.clone()), c.endpoints.as_ref().map(|e| e.1.clone()));
        if row.get::<_, String>(0)? != c.family.label()
            || row.get::<_, String>(1)? != c.source
            || endpoints != proposed
        {
            return Err(reject("claim type/source/endpoints are immutable"));
        }
    }
    let existing: i64 =
        conn.query_row("SELECT count(*) FROM assertions WHERE claim=?1", [&c.claim], |r| r.get(0))?;
    if existing > 0 && matches!(c.intent, Intent::Initial) {
        return Err(reject("Initial cannot replace an existing claim"));
    }
    if existing > 0 && matches!(c.intent, Intent::Revision) {
        let current = current_claim(conn, &c.claim)?;
        if !current.iter().any(|(_, old, _)| {
            old.valid_from == c.valid_from
                && old.valid_to == c.valid_to
                && !matches!(old.intent, Intent::Retraction)
        }) {
            return Err(reject("Revision must preserve one existing accepted fragment interval"));
        }
    }
    Ok(())
}

fn insert_assertion(conn: &Connection, c: &Change) -> Result<i64> {
    conn.execute("INSERT OR IGNORE INTO claim_admission VALUES(?1,1)", [&c.claim])?;
    conn.execute(
        "INSERT INTO assertions(claim,body,family,properties,source,from_id,to_id,vf,vt,kf,intent)
        VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            c.claim,
            c.body,
            c.family.label(),
            serde_json::to_string(&c.properties)?,
            c.source,
            c.endpoints.as_ref().map(|e| &e.0),
            c.endpoints.as_ref().map(|e| &e.1),
            c.valid_from,
            c.valid_to,
            c.known,
            c.intent.label()
        ],
    )?;
    if matches!(c.intent, Intent::Retraction) {
        conn.execute("UPDATE assertions SET accepted=0 WHERE id=last_insert_rowid()", [])?;
    }
    let version = conn.last_insert_rowid();
    insert_projection(conn, version, c.family, "", &c.body)?;
    if c.family == Family::Node {
        for (name, text) in &c.properties {
            insert_projection(conn, version, Family::Property, name, text)?;
        }
    }
    Ok(version)
}

fn insert_projection(
    conn: &Connection,
    version: i64,
    family: Family,
    name: &str,
    text: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO projections(version,family,name,text) VALUES(?1,?2,?3,?4)",
        params![version, family.label(), name, text],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        &format!("INSERT INTO all_{}(rowid,text) VALUES(?1,?2)", family.label()),
        params![id, text],
    )?;
    Ok(())
}

fn remove_current_claim(conn: &Connection, claim: &str) -> Result<()> {
    for family in Family::all() {
        conn.execute(
            &format!(
                "DELETE FROM current_{} WHERE rowid IN
            (SELECT p.id FROM projections p JOIN assertions a ON a.id=p.version WHERE a.claim=?1)",
                family.label()
            ),
            [claim],
        )?;
    }
    Ok(())
}

fn eligible_sql() -> &'static str {
    "EXISTS(SELECT 1 FROM claim_admission ca WHERE ca.claim=a.claim AND ca.active=1) AND a.accepted=1 AND a.vf<=?1 AND (a.vt IS NULL OR ?1<a.vt)
     AND ((?2 IS NULL AND a.kt IS NULL) OR (?2 IS NOT NULL AND a.kf<=?2 AND (a.kt IS NULL OR ?2<a.kt)))
     AND s.allowed=1 AND s.active=1 AND s.valid_from<=?1 AND (s.valid_to IS NULL OR ?1<s.valid_to)"
}

fn documents(
    conn: &Connection,
    family: Family,
    valid: i64,
    known: Option<i64>,
) -> Result<Vec<Document>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT p.id,a.claim,a.id,a.body,p.name,p.text,a.source,a.intent,a.vf,a.vt,a.kf,a.kt,a.accepted
        FROM projections p JOIN assertions a ON a.id=p.version JOIN sources s ON s.id=a.source
        WHERE p.family=?3 AND {} ORDER BY a.claim,a.id,p.name",
        eligible_sql()
    ))?;
    let rows = stmt
        .query_map(params![valid, known, family.label()], |r| {
            Ok(Document {
                id: r.get(0)?,
                claim: r.get(1)?,
                version: r.get(2)?,
                body: r.get(3)?,
                name: r.get(4)?,
                text: r.get(5)?,
                provenance: read_provenance(r)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn refresh_current_claim(conn: &Connection, claim: &str) -> Result<()> {
    let now = conn.query_row("SELECT now FROM config", [], |r| r.get::<_, i64>(0))?;
    for family in Family::all() {
        conn.execute(
            &format!(
                "INSERT INTO current_{}(rowid,text)
            SELECT p.id,p.text FROM projections p JOIN assertions a ON a.id=p.version
            JOIN sources s ON s.id=a.source WHERE a.claim=?3 AND p.family=?4 AND {}",
                family.label(),
                eligible_sql()
            ),
            params![now, Option::<i64>::None, claim, family.label()],
        )?;
    }
    Ok(())
}

fn match_expression(query: &str) -> Result<String> {
    let words = query.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() || words.len() % 2 == 0 {
        return Err(reject("expected ASCII words separated by OR"));
    }
    for (i, word) in words.iter().enumerate() {
        if (i % 2 == 1 && *word != "OR")
            || (i % 2 == 0 && (!word.chars().all(|c| c.is_ascii_alphanumeric()) || *word == "OR"))
        {
            return Err(reject("unsupported query grammar"));
        }
    }
    Ok(words.iter().step_by(2).map(|w| format!("\"{w}\"")).collect::<Vec<_>>().join(" OR "))
}

fn native_search(
    conn: &Connection,
    table: &str,
    query: &str,
    docs: &[Document],
    name: Option<&str>,
    limit: usize,
) -> Result<Vec<Hit>> {
    let by_id = docs.iter().map(|d| (d.id, d)).collect::<BTreeMap<_, _>>();
    let mut stmt =
        conn.prepare(&format!("SELECT rowid,bm25({table}) FROM {table} WHERE {table} MATCH ?1"))?;
    let rows = stmt.query_map([match_expression(query)?], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
    })?;
    let mut hits = Vec::new();
    for row in rows {
        let (id, score) = row?;
        if let Some(d) = by_id.get(&id) {
            if name.is_none_or(|n| n == d.name) {
                hits.push(Hit {
                    claim: d.claim.clone(),
                    version: d.version,
                    body: d.body.clone(),
                    score,
                    provenance: d.provenance.clone(),
                });
            }
        }
    }
    order_hits(&mut hits, limit);
    Ok(hits)
}

#[allow(clippy::too_many_arguments)]
fn search_on(
    conn: &Connection,
    query: &str,
    family: Family,
    name: Option<&str>,
    limit: usize,
    view: View,
    backend: Backend,
) -> Result<Vec<Hit>> {
    match_expression(query)?;
    let docs = documents_view(conn, family, view)?;
    match backend {
        Backend::Materialized => materialized_search(query, &docs, name, limit),
        Backend::Custom => custom_search(conn, query, family, &docs, name, limit),
        Backend::Partitioned => {
            let table = format!("current_{}", family.label());
            let indexed = {
                let mut stmt = conn.prepare(&format!("SELECT rowid FROM {table}"))?;
                let ids = stmt
                    .query_map([], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<BTreeSet<_>>>()?;
                ids
            };
            let selected = docs.iter().map(|d| d.id).collect::<BTreeSet<_>>();
            if selected == indexed {
                native_search(conn, &table, query, &docs, name, limit)
            } else {
                custom_search(conn, query, family, &docs, name, limit)
            }
        }
    }
}

fn materialized_search(
    query: &str,
    docs: &[Document],
    name: Option<&str>,
    limit: usize,
) -> Result<Vec<Hit>> {
    let mut memory = Connection::open_in_memory()?;
    memory.execute_batch("PRAGMA temp_store=MEMORY;
        CREATE VIRTUAL TABLE selected USING fts5(text, tokenize='porter unicode61 remove_diacritics 2');")?;
    {
        let tx = memory.transaction()?;
        {
            let mut insert = tx.prepare("INSERT INTO selected(rowid,text) VALUES(?1,?2)")?;
            for doc in docs {
                insert.execute(params![doc.id, doc.text])?;
            }
        }
        tx.commit()?;
    }
    let hits = native_search(&memory, "selected", query, docs, name, limit)?;
    drop(memory);
    Ok(hits)
}

fn normalized_query_terms(query: &str) -> Result<Vec<String>> {
    match_expression(query)?;
    let memory = Connection::open_in_memory()?;
    memory.execute_batch("PRAGMA temp_store=MEMORY;
        CREATE VIRTUAL TABLE tokens USING fts5(text, tokenize='porter unicode61 remove_diacritics 2');
        CREATE VIRTUAL TABLE vocabulary USING fts5vocab(tokens,'instance');")?;
    for (index, word) in query.split_whitespace().step_by(2).enumerate() {
        memory.execute(
            "INSERT INTO tokens(rowid,text) VALUES(?1,?2)",
            params![index as i64 + 1, word],
        )?;
    }
    let mut stmt = memory.prepare("SELECT term FROM vocabulary ORDER BY doc,offset")?;
    let terms =
        stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(terms)
}

fn custom_search(
    conn: &Connection,
    query: &str,
    family: Family,
    docs: &[Document],
    name: Option<&str>,
    limit: usize,
) -> Result<Vec<Hit>> {
    let terms = normalized_query_terms(query)?;
    if docs.is_empty() {
        return Ok(Vec::new());
    }
    let by_id = docs.iter().map(|d| (d.id, d)).collect::<BTreeMap<_, _>>();
    let mut lengths = docs.iter().map(|d| (d.id, 0usize)).collect::<BTreeMap<_, _>>();
    let term_set = terms.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut frequencies: BTreeMap<(i64, String), usize> = BTreeMap::new();
    let mut df: BTreeMap<String, usize> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT doc,term,count(*) FROM vocab_{} GROUP BY doc,term",
            family.label()
        ))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id = row.get::<_, i64>(0)?;
            if let Some(length) = lengths.get_mut(&id) {
                let term = row.get::<_, String>(1)?;
                let count = row.get::<_, i64>(2)? as usize;
                *length += count;
                if term_set.contains(term.as_str()) {
                    frequencies.insert((id, term.clone()), count);
                    *df.entry(term).or_default() += 1;
                }
            }
        }
    }
    let average = lengths.values().sum::<usize>() as f64 / docs.len() as f64;
    let mut hits = Vec::new();
    let mut stmt = conn.prepare(&format!(
        "SELECT rowid FROM all_{} WHERE all_{} MATCH ?1",
        family.label(),
        family.label()
    ))?;
    let rows = stmt.query_map([match_expression(query)?], |r| r.get::<_, i64>(0))?;
    for row in rows {
        let id = row?;
        let Some(doc) = by_id.get(&id) else {
            continue;
        };
        if name.is_some_and(|n| n != doc.name) {
            continue;
        }
        let mut score = 0.0;
        for term in &terms {
            let tf = *frequencies.get(&(id, term.clone())).unwrap_or(&0) as f64;
            if tf == 0.0 {
                continue;
            }
            let count = *df.get(term).unwrap_or(&0) as f64;
            let idf = ((docs.len() as f64 - count + 0.5) / (count + 0.5)).ln().max(1e-6);
            let norm = lengths[&id] as f64 / average;
            score -= idf * tf * 2.2 / (tf + 1.2 * (0.25 + 0.75 * norm));
        }
        hits.push(Hit {
            claim: doc.claim.clone(),
            version: doc.version,
            body: doc.body.clone(),
            score,
            provenance: doc.provenance.clone(),
        });
    }
    order_hits(&mut hits, limit);
    Ok(hits)
}

fn order_hits(hits: &mut Vec<Hit>, limit: usize) {
    hits.sort_by(|a, b| {
        a.score.total_cmp(&b.score).then(a.claim.cmp(&b.claim)).then(a.version.cmp(&b.version))
    });
    hits.truncate(limit);
}

fn percentiles(samples: &[f64]) -> Result<(f64, f64)> {
    if samples.is_empty() || samples.iter().any(|s| !s.is_finite() || *s < 0.0) {
        return Err(reject("invalid measurement samples"));
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |p: f64| sorted[((p * sorted.len() as f64).ceil() as usize).saturating_sub(1)];
    Ok((rank(0.5), rank(0.95)))
}

impl Backend {
    fn label(self) -> &'static str {
        match self {
            Self::Partitioned => "partitioned",
            Self::Custom => "custom",
            Self::Materialized => "materialized",
        }
    }
    fn all() -> [Self; 3] {
        [Self::Partitioned, Self::Custom, Self::Materialized]
    }
}

struct Workload {
    records: usize,
    history: usize,
    scope: &'static str,
    view: View,
    family: Family,
    name: Option<&'static str>,
}

fn compare_results(actual: &[Hit], expected: &[Hit]) -> Result<f64> {
    if actual.len() != expected.len() {
        return Err(reject("benchmark ranker result count mismatch"));
    }
    let mut delta = 0.0f64;
    for (a, b) in actual.iter().zip(expected) {
        if a.claim != b.claim || a.version != b.version || a.body != b.body {
            return Err(reject("benchmark ranker identity/order mismatch"));
        }
        delta = delta.max((a.score - b.score).abs());
    }
    if delta >= 1e-12 {
        return Err(reject("benchmark ranker score mismatch"));
    }
    Ok(delta)
}

fn measure(store: &Store, w: Workload, samples: usize) -> Result<Vec<serde_json::Value>> {
    let query = "alpha OR beta";
    let reference =
        store.search_backend(query, w.family, w.name, 10, w.view, Backend::Materialized)?;
    let mut times = [Vec::new(), Vec::new(), Vec::new()];
    let mut deltas = [0.0f64; 3];
    for backend in Backend::all() {
        compare_results(
            &store.search_backend(query, w.family, w.name, 10, w.view, backend)?,
            &reference,
        )?;
    }
    for sample in 0..samples {
        for offset in 0..3 {
            let index = (sample + offset) % 3;
            let start = std::time::Instant::now();
            let actual =
                store.search_backend(query, w.family, w.name, 10, w.view, Backend::all()[index])?;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            deltas[index] = deltas[index].max(compare_results(&actual, &reference)?);
            times[index].push(elapsed);
        }
    }
    let docs = documents_view(&store.conn, w.family, w.view)?;
    let current = current_ids(&store.conn, w.family)?;
    let certified = current == docs.iter().map(|d| d.id).collect::<BTreeSet<_>>();
    let mut native_times = Vec::new();
    if certified {
        for _ in 0..samples {
            let start = std::time::Instant::now();
            let actual = native_search(
                &store.conn,
                &format!("current_{}", w.family.label()),
                query,
                &docs,
                w.name,
                10,
            )?;
            native_times.push(start.elapsed().as_secs_f64() * 1000.0);
            compare_results(&actual, &reference)?;
        }
    }
    let native_attribution = if certified {
        let (p50, p95) = percentiles(&native_times)?;
        serde_json::json!({"p50_ms":p50,"p95_ms":p95,"includes_membership_selection":false})
    } else {
        serde_json::Value::Null
    };
    let mut rows = Vec::new();
    for (index, backend) in Backend::all().iter().enumerate() {
        let (p50, p95) = percentiles(&times[index])?;
        rows.push(serde_json::json!({"records":w.records,"history":w.history,"scope":w.scope,"family":w.family.label(),
            "backend":backend.label(),"route":match backend {Backend::Partitioned if certified=>"native",Backend::Partitioned=>"custom_fallback",Backend::Custom=>"custom",Backend::Materialized=>"private_memory"},
            "samples":samples,"p50_ms":p50,"p95_ms":p95,"max_score_delta":deltas[index],
            "selected_documents":docs.len(),"top_claims":reference.iter().map(|h|h.claim.as_str()).collect::<Vec<_>>(),
            "reference_scores":reference.iter().map(|h|h.score).collect::<Vec<_>>(),
            "native_rank_only_attribution":if *backend==Backend::Partitioned {native_attribution.clone()} else {serde_json::Value::Null}}));
    }
    Ok(rows)
}

fn current_ids(conn: &Connection, family: Family) -> Result<BTreeSet<i64>> {
    let mut stmt = conn.prepare(&format!("SELECT rowid FROM current_{}", family.label()))?;
    let ids =
        stmt.query_map([], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok(ids)
}

fn workload_change(
    id: &str,
    body: &str,
    family: Family,
    known: i64,
    source: &str,
    endpoints: Option<(String, String)>,
) -> Change {
    Change {
        claim: id.into(),
        body: body.into(),
        family,
        known,
        source: source.into(),
        endpoints,
        properties: BTreeMap::from([
            ("summary".into(), body.into()),
            ("notes".into(), "auxiliary alpha beta baseline property".into()),
        ]),
        valid_from: 0,
        valid_to: None,
        intent: Intent::Revision,
    }
}

fn seed(path: &Path, records: usize, history: usize) -> Result<(Store, i64, f64)> {
    let mut store = Store::open(path, 50)?;
    store.configure_source("short", true, true, 0, Some(60))?;
    let start = std::time::Instant::now();
    let mut known = 0i64;
    for index in 0..records {
        let node = format!("node-{index:06}");
        let edge = format!("edge-{index:06}");
        let source = if index % 2 == 0 { "short" } else { "source" };
        let endpoints = Some((node.clone(), format!("node-{:06}", (index + 1) % records)));
        for revision in 0..history {
            let old = "alpha oldword repeated historical payload ".repeat(3 + revision % 5);
            known += 1;
            store.put(workload_change(&node, &old, Family::Node, known, source, None))?;
            known += 1;
            store.put(workload_change(
                &edge,
                &old,
                Family::Edge,
                known,
                source,
                endpoints.clone(),
            ))?;
        }
        let body = match index % 4 {
            0 => "alpha",
            1 => "alpha alpha bridge",
            2 => "beta beta extra",
            _ => "gamma filler",
        };
        known += 1;
        store.put(workload_change(&node, body, Family::Node, known, source, None))?;
        known += 1;
        store.put(workload_change(&edge, body, Family::Edge, known, source, endpoints))?;
    }
    Ok((store, known, start.elapsed().as_secs_f64() * 1000.0))
}

fn storage(
    store: &Store,
    records: usize,
    history: usize,
    write_ms: f64,
) -> Result<serde_json::Value> {
    let mut current = serde_json::Map::new();
    let mut all = serde_json::Map::new();
    for family in Family::all() {
        for (prefix, map) in [("current", &mut current), ("all", &mut all)] {
            let count = store.conn.query_row(
                &format!("SELECT count(*) FROM {prefix}_{}", family.label()),
                [],
                |r| r.get::<_, i64>(0),
            )?;
            map.insert(family.label().into(), count.into());
        }
    }
    store.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    let pages = store.conn.query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))?;
    let page_size = store.conn.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))?;
    Ok(serde_json::json!({"records":records,"history":history,"seed_write_ms":write_ms,
        "combined_database_bytes":pages*page_size,"current_rows":current,"all_rows":all}))
}

fn sweep(records: &[usize], history: &[usize], samples: usize) -> Result<serde_json::Value> {
    if records.is_empty() || history.is_empty() || records.contains(&0) || samples == 0 {
        return Err(reject("nonempty axes and positive records/samples required"));
    }
    let dir = tempfile::tempdir()?;
    let mut cells = Vec::new();
    let mut storage_rows = Vec::new();
    for &count in records {
        for &revisions in history {
            let (store, _, write_ms) = seed(
                &dir.path().join(format!("baseline-{count}-{revisions}.sqlite")),
                count,
                revisions,
            )?;
            storage_rows.push(storage(&store, count, revisions, write_ms)?);
            for family in Family::all() {
                cells.extend(measure(
                    &store,
                    Workload {
                        records: count,
                        history: revisions,
                        scope: "current",
                        view: View::Snapshot { valid: 50, known: None },
                        family,
                        name: if family == Family::Property { Some("summary") } else { None },
                    },
                    samples,
                )?);
            }
        }
    }
    let diagnostic_history = if history.contains(&4) { 4 } else { history[0] };
    let (mut store, known, write_ms) =
        seed(&dir.path().join("diagnostic.sqlite"), records[0], diagnostic_history)?;
    let mut correction =
        workload_change("node-000000", "beta corrected", Family::Node, known + 1, "short", None);
    correction.valid_from = 20;
    correction.valid_to = Some(40);
    correction.intent = Intent::Correction;
    let diagnostic_write = std::time::Instant::now();
    store.put(correction)?;
    let mut future =
        workload_change("future", "alpha future", Family::Node, known + 2, "source", None);
    future.valid_from = 60;
    store.put(future)?;
    let diagnostic_write_ms = write_ms + diagnostic_write.elapsed().as_secs_f64() * 1000.0;
    for (scope, view) in [
        ("corrected_history", View::Snapshot { valid: 30, known: None }),
        ("prior_belief", View::Snapshot { valid: 30, known: Some(known) }),
        ("clock_fallback", View::Snapshot { valid: 60, known: None }),
    ] {
        cells.extend(measure(
            &store,
            Workload {
                records: records[0],
                history: diagnostic_history,
                scope,
                view,
                family: Family::Node,
                name: None,
            },
            samples,
        )?);
    }
    let diagnostic_storage = storage(&store, records[0], diagnostic_history, diagnostic_write_ms)?;
    let source_id =
        store.conn.query_row("SELECT sqlite_source_id()", [], |r| r.get::<_, String>(0))?;
    Ok(
        serde_json::json!({"format":"current-history-poc-v1","sqlite_version":rusqlite::version(),"sqlite_source_id":source_id,
        "records_axis":records,"history_axis":history,"samples":samples,"limit":10,"query":"alpha OR beta",
        "timing":"end-to-end including selection, certification/statistics, corpus build, ranking, hydration, drop; warm serial rotated order",
        "storage_scope":"combined canonical + all three all-version/current projection families; not per-backend footprint",
        "storage":storage_rows,"diagnostic_storage":diagnostic_storage,"cells":cells}),
    )
}

/// Run the fixed synthetic sweep, accepting `--records`, `--history`, `--samples`.
/// Emits one JSON report to stdout; invalid arguments or parity failures return errors.
pub fn run_benchmark() -> Result<()> {
    let mut records = vec![128, 512];
    let mut history = vec![0, 4, 16];
    let mut samples = 15;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or_else(|| reject("missing option value"))?;
        match arg.as_str() {
            "--records" => {
                records =
                    value.split(',').map(str::parse).collect::<std::result::Result<Vec<_>, _>>()?
            }
            "--history" => {
                history =
                    value.split(',').map(str::parse).collect::<std::result::Result<Vec<_>, _>>()?
            }
            "--samples" => samples = value.parse()?,
            _ => return Err(reject("unknown option")),
        }
    }
    println!("{}", serde_json::to_string_pretty(&sweep(&records, &history, samples)?)?);
    Ok(())
}

#[cfg(test)]
#[path = "../tests/hypotheses.rs"]
mod hypotheses;
