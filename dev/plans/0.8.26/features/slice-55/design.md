---
title: FathomDB 0.8.26 Slice 55 — SDK parity oracle design
status: DRAFT
target_release: 0.8.26
---

# Slice 55 design — canonical-operation parity

## Model

The shared governed-surface manifest is the spelling map, not proof by itself.
Each entry represents one canonical operation and carries its Python spelling,
TypeScript spelling, and lifecycle state. Only `live` entries participate in
the executable parity set; `reserved` entries cannot satisfy a missing live
operation.

Each binding independently introspects its actual command surface, subtracts a
small documented non-command exclusion set, and resolves every remaining name
through the manifest. The oracle then requires:

1. every introspected name maps exactly once;
2. every live manifest spelling is introspected on its named binding;
3. neither binding exposes an ungoverned command;
4. the canonical live-operation sets are equal; and
5. spelling uniqueness holds within and across the declared binding fields.

This separates three facts that a flat allowlist conflates: canonical
operation identity, binding-specific spelling, and whether an entry is live.

## Failure model

The tests must fail independently for a one-sided removal, one-sided addition,
misspelling, duplicate mapping, and reserved entry used as a live substitute.
Fixture mutation is isolated from production files and does not regenerate a
golden oracle. Error output names the binding, spelling, and canonical
operation responsible for the mismatch.

## Compatibility

The slice preserves the approved live canonical set byte-for-byte at the
operation level. It changes no runtime implementation or public signature. The
five-name recovery denylist remains a separate negative guard; `doctor` remains
absent because it is not a live SDK operation.
