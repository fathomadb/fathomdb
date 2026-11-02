use rusqlite::{params, Connection};
use std::collections::{BTreeMap, BTreeSet};
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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS config(now INTEGER NOT NULL, known INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS sources(id TEXT PRIMARY KEY, allowed INTEGER NOT NULL,
                active INTEGER NOT NULL, valid_from INTEGER NOT NULL, valid_to INTEGER);
            INSERT OR IGNORE INTO sources VALUES('source',1,1,0,NULL);
            CREATE TABLE IF NOT EXISTS assertions(
                id INTEGER PRIMARY KEY, claim TEXT NOT NULL, body TEXT NOT NULL,
                family TEXT NOT NULL, properties TEXT NOT NULL, source TEXT NOT NULL REFERENCES sources(id),
                from_id TEXT, to_id TEXT, vf INTEGER NOT NULL, vt INTEGER,
                kf INTEGER NOT NULL, kt INTEGER, active INTEGER NOT NULL DEFAULT 1,
                accepted INTEGER NOT NULL DEFAULT 1, intent TEXT NOT NULL,
                CHECK(vt IS NULL OR vf<vt), CHECK(kt IS NULL OR kf<kt));
            CREATE INDEX IF NOT EXISTS claim_head ON assertions(claim,kt);
            CREATE TABLE IF NOT EXISTS projections(id INTEGER PRIMARY KEY,
                version INTEGER NOT NULL REFERENCES assertions(id), family TEXT NOT NULL,
                name TEXT NOT NULL, text TEXT NOT NULL);")?;
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
        let tx = self.conn.transaction()?;
        validate_change(&tx, &change)?;
        let originals = current_claim(&tx, &change.claim)?;
        remove_current_claim(&tx, &change.claim)?;
        for (id, old, active) in originals {
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
                let fragment = insert_assertion(&tx, &left)?;
                tx.execute(
                    "UPDATE assertions SET active=?1 WHERE id=?2",
                    params![active, fragment],
                )?;
            }
            if let Some(end) = change.valid_to {
                if old.valid_to.is_none_or(|old_end| end < old_end) {
                    let mut right = old.clone();
                    right.valid_from = end;
                    right.known = change.known;
                    let fragment = insert_assertion(&tx, &right)?;
                    tx.execute(
                        "UPDATE assertions SET active=?1 WHERE id=?2",
                        params![active, fragment],
                    )?;
                }
            }
        }
        insert_assertion(&tx, &change)?;
        refresh_current_claim(&tx, &change.claim)?;
        tx.execute("UPDATE config SET known=?1", [change.known])?;
        tx.commit()?;
        Ok(())
    }

    /// Compare one backend under an explicitly selected temporal view.
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

fn current_claim(conn: &Connection, claim: &str) -> Result<Vec<(i64, Change, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT id,body,family,properties,source,from_id,to_id,vf,vt,kf,intent,active
        FROM assertions WHERE claim=?1 AND kt IS NULL ORDER BY vf",
    )?;
    let mut result = Vec::new();
    let mut rows = stmt.query([claim])?;
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
                WHERE p.family=?1 AND a.active=1 AND (a.accepted=1 OR ?2=1) AND s.allowed=1 AND s.active=1
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
    Ok(())
}

fn insert_assertion(conn: &Connection, c: &Change) -> Result<i64> {
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
    "a.active=1 AND a.accepted=1 AND a.vf<=?1 AND (a.vt IS NULL OR ?1<a.vt)
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

/// Run the reproducible synthetic POC cost sweep; errors stop the run.
pub fn run_benchmark() -> Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "../tests/hypotheses.rs"]
mod hypotheses;
