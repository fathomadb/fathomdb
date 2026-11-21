---
title: Pinned-override rot guard
status: ACTIVE
---

# Pinned-override rot guard

A dependency pin added to *fix* an advisory can silently become the *cause* of
the next one. The repository therefore enforces a mechanical guard, because the
failure is invisible by construction: the pin still looks like a fix, and its
comment still says so.

## The case that motivated it

`package.json` carried this, added in 0.8.9:

```json
"comment-overrides": "0.8.9 R-DEP-1: force transitive security patches for
  markdown-it (GHSA-6v5v-wf23-fmfq) and js-yaml (GHSA-h67p-54hq-rp68) …",
"overrides": { "markdown-it": "14.2.0", "js-yaml": "4.2.0" }
```

On 2026-08-04, GHSA-52cp-r559-cp3m landed against js-yaml with vulnerable range
`>= 4.0.0, < 4.3.0`. **The pinned 4.2.0 was inside it.** The line written to
close a js-yaml advisory had become the js-yaml exposure, while its comment
still advertised it as the remedy. Nothing in the repository disagreed with that
comment.

Three properties make this a guard-shaped problem rather than a
remember-to-check problem:

1. **The artifact lies with confidence.** A reader auditing `overrides` sees a
   named GHSA and a justification. Nothing signals that the justification has
   expired.
2. **A pin defeats the normal remedy.** `overrides` forces a version regardless
   of what upstream ships, so the usual "a patched release exists, take it"
   path does not apply. The pin must be revisited *by hand* or not at all.
3. **The detection channel was off.** Dependabot PR creation is paused
   (`.github/dependabot.yml`), so the advisory produced no pull request. It
   surfaced only as a push-time banner, which is not a gate.

A second, quieter rot exists in the same block: the comment says *"Remove when
markdownlint-cli2 ships patched deps."* Nothing checks whether that day has
come, so a pin outlives its reason and silently holds the tree back.

## What the guard must assert

- **R1 — no pin is itself vulnerable.** For every entry in `overrides`, the
  pinned version must not fall in any known advisory's vulnerable range. This is
  the js-yaml case, and it is the load-bearing requirement.
- **R2 — no pin is obsolete.** If the natural resolution *without* the override
  would already satisfy every advisory and the range the dependents ask for,
  the pin has outlived its reason and should be reported so it can be removed
  deliberately.
- **R3 — every pin states why it exists.** A pin without a recorded reason
  cannot be evaluated by a future reader. The existing `comment-overrides`
  convention is the shape; the guard should require *a* rationale, not a
  particular prose style.
- **R4 — the Cargo exceptions are reproducible and explicitly bounded.**
  [The checker-owned contract](../../scripts/pinned-override-rot.json) names the
  exact Slice60 candidate cohort: `cudarc-fathomdb =0.21.0`, the Candle fork
  trio `=0.13.0`, and `candle-kernels-fathomdb =0.13.0`. The kernel and trio
  share one full owner-fork code revision; cudarc has its own qualified revision.
  Both the root and standalone SDK driver repeat the complete exact patch
  cohort, including the Candle-file-source kernel override. Both tracked locks
  must operationally resolve those exact versions, logical URLs and revisions once each.
  The kernel uses the owner HTTPS source; the other four entries retain the
  governed local logical Git sources. No registry checksum belongs on these
  Git package entries. A missing entry, mixed family, source/checksum/revision
  disagreement, or unqualified revision fails. The static checker validates the root lock; actual locked metadata checks
  separately qualify the standalone lock. Both locks are reconciled by
  Cargo under each workspace's selected features; byte equality with the old
  release lock is not asserted.

  Repository `.cargo/config.toml` and `.cargo/config` remain fail-closed:
  `[patch]` tables and unparseable candidates fail. The four exact `[source]`
  tables mirror the two logical file URLs to owner HTTPS URLs at the same
  immutable revisions. Extra keys, routing, revisions or replacements fail;
  ordinary non-source configuration remains valid. Any other manifest
  `[patch]`, `[replace]`, or direct Git dependency remains unsupported,
  including `libsqlite3-sys`, which has used the registry since Slice42.
  This establishes qualified source consumption, not public registry delivery.
  Old published 0.12/0.20 artifacts cannot carry changed source under their
  immutable registry identity. The new versions and renamed kernel are
  qualified sources; owner seq413 authorized matching public publication, whose
  actual checksums and consumer outcome are recorded in Slice60 status.

## Design constraints

**Must not be advisory-only.** The repository already learned that a
permanently-red or purely-informational gate trains readers to discount it. This
either gates or is not worth building.

**Must not require network access at verify time.** `verify` runs offline-ish
and must stay deterministic and fast. Prefer resolving against the lockfile plus
a checked-in advisory snapshot, or gate the network probe behind its own
always-on-but-separate job that is allowed to be the only network consumer.
Whichever is chosen, a network outage must produce a *loud failure*, never a
silent pass — an unreachable advisory database means **unverified**, not
**clean**.

**Must not duplicate `npm audit`.** `npm audit` answers "is the resolved tree
vulnerable". It does **not** answer "is this pin still the right pin", which is
R2, nor does it distinguish a vulnerability the pin *caused* from one it merely
failed to prevent. Use `npm audit` as one input; do not stop there.

**Scope beyond npm, deliberately.** The root Cargo patches are in scope;
Cargo override grammar is not. `cargo_candle_exception` records the Git
exception and its `external-source-unassessed` advisory posture: the checked-in
npm GitHub Advisory snapshot does not support a Rust vulnerability assertion.
It is not a substitute for a Rust advisory scan and must never be phrased as
"no known vulnerabilities"; changing the immutable revision requires a fresh
human advisory review and a deliberate guard redesign.
`cargo_fork_pins` records that exact logical graph, the two governed workspace
manifest paths and the exact source-mirror tables. Workspace manifest patches
supply the fork/kernel overrides; repository Cargo config is source-only and
config patches remain unsupported. HTTPS fetch availability is separate from
offline source qualification. The
registry-only Tegra probe checks the current candidate cohort without root
patches. Its public-registry resolution remains OPEN until those exact package
versions are published; staged normalized archives prove a separate local
consumer route. Historical published-floor receipts retain their original
versions and checksums; they are not evidence of the new candidate's visibility. `scripts/governed-
surface-pin.json` pins repository contract content rather than an external
dependency version, so its integrity remains owned by
`scripts/check-governed-surface-pin.sh`.

## Notes for implementation

- The four advisories that remain open at authoring time (`torch`, `idna`,
  `setuptools` in `python/uv.lock`; `dompurify` in the mermaid tool) are
  **documented as intentionally uncovered** in `.github/dependabot.yml`. The
  guard must not re-litigate accepted risk; an accepted exception needs a place
  to live, or the gate will be turned off within a week.
- Red-first is required: the test must construct a pin that has become
  vulnerable and prove the guard fails on it. Re-pinning js-yaml back to 4.2.0
  in a fixture is the exact historical case and makes the regression concrete.
- Cargo fixtures must prove the narrow exceptions: the approved
  `candle-kernels-fathomdb` patch and exact qualified fork pins pass, but a missing entry, a
  revision or lock drift, or a fork pin with another version, source or a checksum
  fails; a `libsqlite3-sys` or `cudarc` patch, any `[replace]`, and any
  direct Git dependency are unsupported.
- The guard runs where it can act — this is cheap and static, so it belongs in
  the fast, always-on tier alongside the other governance gates, not behind the
  ~35-minute `verify`.
