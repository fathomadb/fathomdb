import json

from runner import job_key, read_done, run_jobs


def work(job):
    if job == ("boom",):
        raise ValueError("planted failure")
    return {"x": job[0], "y": job[0] * 2}


def test_streams_resumes_and_records_errors(tmp_path):
    base = str(tmp_path / "out")
    log = str(tmp_path / "log.txt")
    jobs = [(i,) for i in range(10)] + [("boom",)]
    rows, n_err = run_jobs("t", work, jobs, base, log, procs=2, every=3)
    assert n_err == 1 and len(rows) == 10
    lines = [json.loads(x) for x in open(base + ".jsonl")]
    assert len(lines) == 11 and sum("_error" in x for x in lines) == 1
    err = next(x for x in lines if "_error" in x)
    assert "planted failure" in err["_error"] and "boom" in err["_job"]
    assert "ETA" in open(log).read()
    # resume: nothing re-run except the failed job
    rows2, n_err2 = run_jobs("t", work, jobs, base, log, procs=2)
    assert n_err2 == 1 and len(rows2) == 10
    assert len(open(base + ".jsonl").readlines()) == 12
    assert open(base + ".csv").read().splitlines()[0] == "x,y"


def test_torn_last_line_is_rerun(tmp_path):
    base = str(tmp_path / "out")
    log = str(tmp_path / "log.txt")
    jobs = [(i,) for i in range(4)]
    run_jobs("t", work, jobs[:2], base, log, procs=2)
    with open(base + ".jsonl", "a") as f:
        f.write('{"x": 2, "_key": "' + job_key(jobs[2])[:5])  # crash mid-write
    done, _ = read_done(base + ".jsonl")
    assert len(done) == 2
    rows, _ = run_jobs("t", work, jobs, base, log, procs=2)
    assert sorted(r["x"] for r in rows) == [0, 1, 2, 3]
    assert len(read_done(base + ".jsonl")[0]) == 4  # the row written after the torn line is intact
