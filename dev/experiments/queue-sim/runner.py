"""Crash-safe, resumable, observable job runner for the simulation studies.

- Each finished job is appended to <out>.jsonl as one line and flushed (imap_unordered), so a crash loses
  only the jobs in flight.
- Resume: jobs whose key (sha1 of the job's repr: config + tag) is already in the JSONL are skipped.
- Progress: every `every` jobs a line "name done/total elapsed ETA errors" is appended to the log.
- Errors: a worker exception becomes an error line {"_key", "_error": traceback, "_job": repr} and the pool
  continues. The caller gets the error count; any error is a simulator bug to fix before trusting a run.
- At the end the non-error rows are exported to <out>.csv for analysis.py.
"""
from __future__ import annotations

import csv
import hashlib
import json
import os
import time
import traceback
from multiprocessing import Pool


def job_key(job) -> str:
    return hashlib.sha1(repr(job).encode()).hexdigest()


def _wrap(arg):
    fn, job = arg
    key = job_key(job)
    try:
        row = fn(job)
        row["_key"] = key
        return row
    except Exception:  # noqa: BLE001 - recorded, not swallowed: every error row is a bug to fix
        return {"_key": key, "_error": traceback.format_exc(), "_job": repr(job)[:4000]}


def read_done(jsonl_path):
    done, errors = {}, []
    if os.path.exists(jsonl_path):
        with open(jsonl_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    r = json.loads(line)
                except json.JSONDecodeError:
                    continue  # a torn last line from a crash; that job is re-run
                if "_error" in r:
                    errors.append(r)
                else:
                    done[r["_key"]] = r
    return done, errors


def run_jobs(name, fn, jobs, out_base, log_path, procs=22, every=200, chunksize=1, retry_errors=True):
    """Run fn over jobs; returns (rows, n_errors_this_run). out_base is the path without extension."""
    jsonl = out_base + ".jsonl"
    done, old_errors = read_done(jsonl)
    jobs = list(jobs)
    todo = [j for j in jobs if job_key(j) not in done]
    if not retry_errors:
        bad = {e["_key"] for e in old_errors}
        todo = [j for j in todo if job_key(j) not in bad]
    t0 = time.time()
    n_err = 0
    with open(log_path, "a") as log:
        log.write(f"{time.strftime('%H:%M:%S')} {name}: {len(jobs)} jobs, {len(jobs) - len(todo)} already done, {len(todo)} to run\n")
        log.flush()
        if os.path.exists(jsonl) and os.path.getsize(jsonl):
            with open(jsonl, "rb") as f:
                f.seek(-1, os.SEEK_END)
                torn = f.read(1) != b"\n"
            if torn:  # a crash mid-write: terminate the torn line so the next row starts clean
                with open(jsonl, "a") as f:
                    f.write("\n")
        with open(jsonl, "a") as out, Pool(procs) as pool:
            for i, row in enumerate(pool.imap_unordered(_wrap, [(fn, j) for j in todo], chunksize=chunksize), 1):
                out.write(json.dumps(row, default=str) + "\n")
                out.flush()
                if "_error" in row:
                    n_err += 1
                    log.write(f"{time.strftime('%H:%M:%S')} {name}: ERROR in job {row['_key'][:10]}: "
                              f"{row['_error'].strip().splitlines()[-1]}\n")
                    log.flush()
                else:
                    done[row["_key"]] = row
                if i % every == 0 or i == len(todo):
                    el = time.time() - t0
                    eta = el / i * (len(todo) - i)
                    log.write(f"{time.strftime('%H:%M:%S')} {name}: {i}/{len(todo)} elapsed {el:.0f}s ETA {eta:.0f}s errors {n_err}\n")
                    log.flush()
        log.write(f"{time.strftime('%H:%M:%S')} {name}: finished, {len(done)} rows, {n_err} errors this run\n")
    keys = [job_key(j) for j in jobs]
    rows = [done[k] for k in keys if k in done]
    if rows:
        fields = []
        for r in rows:
            for k in r:
                if k not in fields and k != "_key":
                    fields.append(k)
        with open(out_base + ".csv", "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=fields)
            w.writeheader()
            for r in rows:
                w.writerow({k: v for k, v in r.items() if k != "_key"})
    return rows, n_err
