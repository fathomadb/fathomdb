---
title: FathomDB 0.8.26 Slice 50 — verification architecture
status: DRAFT
---

# Slice 50 design — verification architecture

## Evidence topology

One exact candidate commit produces all artifacts. Each witness runs from a
clean installation and records artifact hash, platform, runtime/toolchain,
database creation version, command/test identity, bounded output, and result.
Cross-SDK tests exchange only public persisted state and public models.

The integrated Memex-shaped flow remains FathomDB-owned contract evidence:

1. atomically write the supported canonical/derived/dependency/edge unit;
2. restart and prove deterministic replay;
3. freeze, search with explanation, and resolve exact ranked evidence;
4. traverse and resolve target/terminal-edge evidence by revision;
5. page and inspect dependency/projection state; and
6. run the version-matched external integrity check under its quiescence rule.

It does not import Memex, choose ontology or contradiction policy, or assert
answer/admission semantics.

## Manifest and decision boundary

The manifest is generated evidence, not a manually asserted green status. It
separates locally executed targets, externally executed targets, skips with
owner/reason, and failures. Slice 50 can establish a release candidate; only a
separate explicit HITL action may tag or publish it.
