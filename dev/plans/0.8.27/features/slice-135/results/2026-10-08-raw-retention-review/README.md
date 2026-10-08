---
title: Slice 135 exact-source raw archive retention review
status: LOCAL_BUNDLE_VERIFIED_PUBLICATION_PENDING
target_release: 0.8.27
---

# Exact-source raw archive retention — 2026-10-08

Eight exact-source raw campaigns remain in their original local directories:
E01–E12 paired and smoke, Python S01/S02/S02-L/S03, and TypeScript S01/S02.
The [audit](raw-retention-audit.json) records each archive path, manifest
hash and entry count. All 4,577 manifest entries passed `sha256sum -c` in
place. Copies of the eight original manifests are retained under
[`manifests/`](manifests/). The measured candidate source is
`224e44c593c13d86ece648adabe445723db04070`.

A 121 MiB [local transfer bundle](phase1-exact-raw.tar.zst) contains copies
of all eight archives. Its [SHA-256 record](BUNDLE-SHA256SUMS) is
`9674227303546aa9a7bffb9152ebc2c25fa4dfeaad6ec89469d455ad2c15ea8e`.
The [bundle index](bundle-index.json) lists the archive names and the three
model assets copied into it. All 63 external model links in the original
engine archives were rewritten only inside the bundle as relative links to
those assets. Extracting the bundle into a separate directory verified all
eight original manifests again and confirmed every link resolves inside the
extracted tree. The original archive bytes and links were not altered.

The configured Gitleaks scanner found zero findings in the six Python and
TypeScript archives. It reported 42 `generic-api-key` matches in the two
engine archives: 40 paired and two smoke. Each match was checked against
its source line and column and is the exact pinned public `tokenizer.json`
SHA-256, not a credential. The audit records the scanner configuration
hash, finding counts, filenames and classification. A prior automatic
approval review rejected changing the path-scoped scanner allowlist for
these raw files. No scanner policy was changed, and the raw bundle is not
tracked or published.

The tracked checkpoint, audits, manifests, bundle index and checksum allow
later verification of this local transfer. A clean Git clone contains those
records but needs the separate bundle to replay raw samples. Publication
remains a separate decision; local retention is verified for this Phase 1
checkpoint.
