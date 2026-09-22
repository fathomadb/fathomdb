# Slice 30 TDD chronology

## RED

The focused comparator contract was added before the repository tool existed.
It covers deterministic fixture capture, unchanged comparison, every required
surface mutation, combined Rust feature-row identity, metadata mismatch, and
duplicate-key rejection. The valid RED command was:

```text
python3 scripts/tests/test_slice30_surface_comparator.py
```

It exited 1 while loading the intentionally absent entry point:

```text
FileNotFoundError: [Errno 2] No such file or directory:
'/home/coreyt/projects/fathomdb-worktrees/release-0.8.27/dev/tools/surface_comparator.py'
```

The production tool and tracked baseline did not exist at this checkpoint.

## GREEN tool contract

The repository tool then implemented the common normalized row model, all six
Rust feature identities, independent Python export/registration/stub adapters,
NAPI and TypeScript declaration adapters, package metadata/runtime exports,
metadata fail-closed comparison, duplicate rejection, and immutable compare.
The focused contract passed:

```text
ok    slice30-surface-comparator
```

The real heavy capture and reviewed baseline remained a separate next step so
the capture-source commit could be clean and contain the tool but no baseline.

## Real-output parser RED

The first real Rust matrix exposed a legitimate `cargo public-api` shape that
the minimal fixture lacked: distinct blanket impls can emit the same associated
type path with different signatures. A focused fixture reproduced the failure:

```text
SurfaceError: duplicate normalized key in rust-engine-default:
type fathomdb_engine::Thing::Error
```

Exact duplicate lines must still fail closed (superseded below: identical
compiler entries now collapse idempotently, and only a manifest with duplicate
normalized keys is rejected); distinct signatures sharing a display path
require deterministic disambiguation.

The comparator now keys row entries by kind, complete path, and a digest of the
normalized signature. The focused suite returned GREEN while its exact-copy
duplicate arm continued to reject ambiguity.

A second real-output pass showed why cargo's surrounding lines are semantic:
two distinct trait impls may expose byte-identical associated method lines. A
fixture with `Debug::fmt` and `Display::fmt` reproduced the remaining exact-line
collision. The adapter must bind associated items to their preceding complete
impl signature, not collapse them or accept context-free duplicates.

The Rust adapter now carries the complete preceding impl signature into each
associated function/type signature. The trait-qualified fixture and the full
mutation suite returned GREEN; a context-free exact duplicate still failed at
this point (later collapsed idempotently, as described next).

The next real run showed rustdoc can repeat the *same* complete impl/item pair
while traversing different public types. A focused repeated-pair fixture failed
before normalization. The intended distinction is now explicit: identical
compiler entries collapse idempotently, while a manifest containing duplicate
normalized keys remains invalid and comparison rejects it.

The adapter now performs that idempotent collapse. The repeated-pair fixture,
all mutation arms, and the duplicate-manifest rejection returned GREEN.

## Initial baseline capture (superseded by review)

Clean implementation commit `3b726cba1700e55486d3b2fbb6a922f439f25d54`
was captured twice with `cargo-public-api 0.52.0`,
`nightly-2026-04-24`, Rust nightly `1.97.0-nightly (36ba2c771
2026-04-23)`, Node `v26.8.2`, and TypeScript `6.0.3` on
`x86_64-unknown-linux-gnu`. Both 12-row files had SHA-256:

```text
a8cee76657af6557c2117fad77e66119314b85ef3b77a4c61d31943f33a694fe
```

Byte comparison and semantic comparison both passed; the semantic result was
`equal: true` with empty metadata and row diffs. Production NAPI generation
used the exact `npm run build:native` command and `default-embedder`. The only
environmental exception was the sandbox denying napi-rs `/bin/sh` spawn; the
unchanged command succeeded through the approved unconfined route.

## Code-review RED and GREEN

The initial code review identified five correctness gaps. RED commit
`f621c9cd` added focused failures for:

- equal surfaces captured from distinct valid Git SHAs;
- invalid provenance rejection independent of semantic equality;
- removal of the PyO3 `m.add("StorageError", ...)` exception alias;
- a native-stub `StorageError` base-class mutation;
- a missing executable becoming a typed `ComparatorError`;
- a near-match `cargo-public-api 10.52.0` version being rejected;
- a hardlink alias being unable to rewrite the reviewed baseline;
- atomic-write failure preserving the prior output and removing its temporary
  file; and
- registration of the focused comparator test in `scripts/agent-test.sh`.

The valid RED command remained:

```text
python3 scripts/tests/test_slice30_surface_comparator.py
```

It exited 1 because the comparator treated the two valid capture SHAs as a
semantic metadata difference. GREEN commit `add4f3f4` separated provenance
validation/reporting from semantic equality; completed the Python adapters;
pinned Node `v25.9.0`; made tool-version checks exact; typed missing-command
errors; added inode-safe output protection and atomic replacement; and wired
the focused test into the canonical repository test harness. The focused
suite and Python compilation then passed.

## Corrected baseline and current-candidate proof

Clean GREEN source commit `add4f3f4f066f0c4b1b47d7a91c1a4f979eba6bf`
was captured twice with `cargo-public-api 0.52.0`,
`nightly-2026-04-24`, Rust nightly `1.97.0-nightly (36ba2c771
2026-04-23)`, Node `v25.9.0`, and TypeScript `6.0.3` on
`x86_64-unknown-linux-gnu`. Both files were byte-identical at SHA-256:

```text
7c76bd0d409cecc5dc073e29baf7806b329379bd9475c7ab138dda12cde5cdf2
```

Tracking commit `d80a66236ca9dc0dd6456e2a0d5322c3722d364c` then produced a
fresh candidate with SHA-256
`c78b89da8c40be4821c9e25fa53dd6a834de79ef4b7d917079a35d0c512c5413`.
The comparator returned `equal: true`, empty metadata and row diffs, and the
two distinct validated provenance SHAs.

The Rust re-export/governed-surface controls passed 4 tests without features
and 5 with `operator`. The TypeScript release-surface route was rerun under the
pinned Node with `RELEASE_SURFACE_TESTS=1`; its production no-test-hook check
executed and passed. TAP reported 2/2, while the default-embedder open branch
returned early under the recorded `FATHOMDB_SKIP_NETWORK_TESTS=1` condition.

## Adversarial review FIX-1

- RED `eb54ca29`: re-exported TypeScript mutations, moved-declaration equality,
  unresolved re-export failure, Python wrapper signature/removal diffs with
  body-edit and moved-definition equality, cfg gating diff, and `build:native`
  drift rejection. Failed before implementation.
- GREEN `4ce45c51`: comparator changes; focused contract passes.
- Baseline recaptured at `4ce45c518789108a9fcc1da35fda4c9b74f19aff`: `24cc30a01539eddc7515072a11c09b45fb9077b2aa34a77b1ed8e7032f4cf3dd`.

## Adversarial review FIX-2

- RED `95441a30`: item-scoped cfg gate, comment-only declaration edits, brace-in-comment,
  and URL string-literal arms failed before implementation (duplicate
  ungated `add_class` compared equal).
- GREEN `58bc8eb4`: cfg blocks open on gated items; declaration comments
  stripped outside string literals.
- Two byte-identical captures at `58bc8eb4`: `b54a01cc486c4d1755297196831f5490d0311237e12ca6f40d37c03af1b3a1b4`.

## Adversarial review Phase 2 FIX-1 (test review)

Test-only hardening; each new arm was shown to fail against a neutered
comparator (impl context dropped, tools/target ignored, dirty-tree check
removed, post-generation re-check removed): Rust impl-context entry counts
and a removed-impl arm; tool/target/schema identity arms; root `__all__`
removal and invalid-`__all__` arms; mocked capture guards (HEAD mismatch,
short SHA, dirty tree, Node pin, TypeScript version parse, unowned scratch,
disk floor, post-generation tracked change); `export type {…} from`, alias,
default-export exclusion, re-export cycle, relative Python import,
unterminated comment, and baseline symlink arms; single-row isolation plus
exact-path checks in `assert_row_diff`; and seeded stdlib-`random` generative
properties for Rust line normalization, canonical JSON round trip, and
comment stripping. Properties use a fixed seed rather than `hypothesis` because
the registered suite runs under bare `python3`, where `hypothesis` is not
guaranteed.

## Adversarial review Phase 3 FIX-1

- RED `a8786164`: local export list, generic/unsafe impl ownership, literal and
  single-line cfg, unbalanced braces, pairing, `const enum`, literal brace,
  missing input, and scratch declaration-emit arms failed before
  implementation.
- GREEN `df801746`; two byte-identical captures: `7db3d883f99940aea69c58079e64d6794be459ef9ab76561106a314e61900384`.
