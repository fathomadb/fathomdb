//! Line-oriented embedded SurrealDB server for the COMP side-plan harness.
//!
//! Each stdin line is a JSON request: `{"op":"open","endpoint":"rocksdb:<path>"}`,
//! `{"op":"query","sql":"...","vars":{"name":"string"}}`, or `{"op":"close"}`.
//! Each reply is one JSON line `{"ok":bool,"ns":u128,"result":...,"error":...}`;
//! `ns` times only the `db.query` call through materialized results.

use std::io::{BufRead, Write};
use std::time::Instant;

use anyhow::{Result, anyhow};
use serde_json::{Value as Json, json};
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};
use surrealdb::types::Value;

async fn query(db: &Surreal<Any>, req: &Json) -> Result<(u128, Json)> {
    let sql = req["sql"].as_str().ok_or_else(|| anyhow!("missing sql"))?;
    let started = Instant::now();
    let mut q = db.query(sql);
    if let Some(vars) = req["vars"].as_object() {
        for (name, value) in vars {
            let text = value.as_str().ok_or_else(|| anyhow!("vars must be strings"))?;
            q = q.bind((name.clone(), text.to_string()));
        }
    }
    let mut response = q.await?;
    let count = response.num_statements();
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let value: Value = response.take(index)?;
        out.push(value.into_json_value());
    }
    Ok((started.elapsed().as_nanos(), Json::Array(out)))
}

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut db: Option<Surreal<Any>> = None;
    for line in stdin.lock().lines() {
        let req: Json = serde_json::from_str(&line?)?;
        let reply = match req["op"].as_str() {
            Some("open") => {
                let endpoint = req["endpoint"].as_str().unwrap_or_default();
                let started = Instant::now();
                match connect(endpoint).await {
                    Ok(handle) => match handle.use_ns("bench").use_db("bench").await {
                        Ok(_) => {
                            db = Some(handle);
                            json!({"ok": true, "ns": started.elapsed().as_nanos(), "result": "3.3.0"})
                        }
                        Err(e) => json!({"ok": false, "error": e.to_string()}),
                    },
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                }
            }
            Some("query") => match db.as_ref() {
                Some(handle) => match query(handle, &req).await {
                    Ok((ns, result)) => json!({"ok": true, "ns": ns, "result": result}),
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                },
                None => json!({"ok": false, "error": "not open"}),
            },
            Some("close") => {
                db = None;
                json!({"ok": true, "ns": 0})
            }
            _ => json!({"ok": false, "error": "unknown op"}),
        };
        writeln!(stdout, "{reply}")?;
        stdout.flush()?;
    }
    Ok(())
}
