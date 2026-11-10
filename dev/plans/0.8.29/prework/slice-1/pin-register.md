# Slice 1 exact pin and qualified baseline register

PROPOSED posture, source inspected November10,2026 initially at dated planning40d82c2e7/product6edc1155f;
Slice0 documentation integration850f2c15d subsequently inspected.
“Retain” preserves a qualified current choice; it does not mean latest, secure,
unyanked or fully licensed today. Source pins are not live installed-tool facts.
10 is receiving dependency/preparation owner unless a row names another slice.
Version ranges below are distinguished from exact pins and resolved locks.

## Runtime, protocol, ABI and source groups

| Group and exact edit/authority sites | Declared → locked/source | PROPOSED disposition, reason and reopen/qualification trigger |
| --- | --- | --- |
| W/E package axes: `Cargo.toml`, crate manifests, `src/python/pyproject.toml`, `src/ts/package.json`, platform packages/locks; `scripts/set-version.sh` | W0.8.28; E0.8.0 independent | Retain until coordinated W decision/execution in10; no lockstep E change without trait change. Preserve exact source/installed version and registry-resolution checks; format37 does not move. |
| SQLite/vector: engine/schema/CLI/MCP manifests, `Cargo.lock`; page-cache ADR | rusqlite0.41 range→0.41.1; libsqlite3-sys0.39.1→SQLite3.54.1; sqlite-vec `=0.1.9` | Retain .28 Slice42 exit. New target unknown/unselected; reopen for applicable advisory/bug/explicit need. Any change requires real SQLite/vector admission/eligibility/deletion, per-connection page-cache build policy, Windows WAL/path and installed Rust consumer proof. No vendored SQLite patch revival. |
| MCP protocol edge: `fathomdb-mcp/Cargo.toml`, `tests/manifest.rs`, architecture | rmcp `=3.5.1` normal+dev→3.5.1; macros3.5.1 | Retain exact protocol/confinement baseline, publish=false product binary. Scoped target research before change; majors need migration/conformance/determinism and both-format qualification in60/80, not an automatic bump. |
| Fork family: `fathomdb-embedder/Cargo.toml`, `scripts/pinned-override-rot.json` | candle-{core,nn,transformers}-fathomdb `=0.11.0`; cudarc-fathomdb `=0.19.10` | Retain qualified registry pins/checksums below. Fork Candle0.11.0 is based on upstream0.10.2; shared version numbers do not imply upstream parity. Reopen only for approved required110/120 repair in140; broader rebase/marker/multiarch is .30 (seq-367). |
| Kernel source: root `Cargo.toml`, benchmark-driver manifest, `Cargo.lock`, override register | candle-kernels0.10.2 at Git `182ceab0547348993b6dbec4b1aed610dcc8b23f` | Retain static CUDART/driverless mapping boundary. Root patch does not propagate to published Rust consumers. Any source promotion needs fork-local qualified code/receipts and fresh source/advisory review before affected110/120/130 installed-product proof. |
| Fork closure: `scripts/cudarc-fathomdb-lock-set.json` | target-specific linux-normal closure; set SHA `66719ccca12c3a1b7ed4781cc31bb1870343f1d4bdce256e1ffde83f8551f318` | Retain; guard projects relevant cudarc closure, not all unrelated Cargo.lock bytes. Rustls update must not mutate required closure/root feature set. Changed closure requires its owned qualification, never metadata regeneration to silence gate. |
| ONNX eval: embedder manifest / Cargo.lock | ort `=2.0.0-rc.10`, ort-sys2.0.0-rc.10; load-dynamic | Retain/investigate future stable target (unknown). Prior rc.12 ort-sys/VitisAI mismatch explains pin. CPU eval only, not production caller-provider/GPU EP. Need runtime library/ABI/eval parity before change; no native download implied. |
| Tokenizers: embedder manifest / Cargo.lock | direct0.20 range→0.20.4; transitive0.22.2 | Retain; model/tokenization identity coupling. Reopen with applicable defect or selected model migration, qualify known tokenization/output identity. Exporter0.19.1 is a different environment. |
| Python ABI: `fathomdb-py/Cargo.toml` / lock | PyO3 0.29 range→0.29.0; abi3-py310 | Retain package>=3.10 contract. Changes need supported installed-wheel/lifecycle/error/byte parity, no floor change by inference. |
| JS ABI/tool: `fathomdb-napi/Cargo.toml`, `src/ts/package.json` / locks | napi2→2.16.17; derive2→2.16.13; build2→2.3.2; @napi-rs/cli ^2.18.4→2.18.4 | Retain; major migration is coupled Rust/CLI/loader/platform package work and unselected. Trigger explicit need or applicable advisory, with package/SDK proof. |
| Downloader test fixture server: `fathomdb-embedder/Cargo.toml` / lock | httpmock `=0.8.3`→0.8.3 (dev dependency) | Retain/investigate original exact-pin rationale, currently undocumented. Test-tool exposure, not product runtime. Reopen for applicable advisory or test-server compatibility need; downloader fixture semantics and deterministic retry/hash/cache tests must survive. No database mock is introduced. |
| Windows paths: engine manifest / `connection_runtime.rs` | dunce `=1.0.5`→1.0.5 | Retain/investigate exact-pin rationale; actual use is Windows local path simplification for WAL. Unknown original pin reason is not license to remove. Reopen with primary rationale/fix and Windows local-verbatim/WAL regression proof. |
| Optional download TLS: embedder manifest / `Cargo.lock` | ureq2 tls-only→2.12.1; rustls0.23.40; webpki0.103.13 | Approved contained10 update to rustls0.23.46/necessary webpki0.103.16; current-lock adaptation and selected feature/configuration proof required. No query egress change. |

Fork registry authenticity is source metadata, not a fresh registry query:

| Exact registry package | Version | Published VCS SHA | Cargo checksum retained |
| --- | --- | --- | --- |
| `cudarc-fathomdb` | 0.19.10 | `b777f8d94eedca8334e9331cf520d554b22544b7` | `8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2` |
| `candle-core-fathomdb` | 0.11.0 | `182ceab0547348993b6dbec4b1aed610dcc8b23f` | `b5feacc063f44c77a54cb8b43d03c5e32fa5421f4163f0524194783e21563341` |
| `candle-nn-fathomdb` | 0.11.0 | `182ceab0547348993b6dbec4b1aed610dcc8b23f` | `69c7432aa63cb543065f49453fa2e258e4dc3bd458abbcb7c150d17e90026200` |
| `candle-transformers-fathomdb` | 0.11.0 | `182ceab0547348993b6dbec4b1aed610dcc8b23f` | `513008c79a9751ec5fdc08918dcdb1a8872c95e90f7697a9f8c5ef1951e3d222` |

`pinned-override-rot.json` names the fork-local publication receipts; their prior
qualification is inherited .28 evidence, not rerun in1. The kernel source is
explicitly external-source-unassessed by the checked-in npm snapshot. The
`governed-surface-pin.json` is a signed contract-content pin, not a dependency
advisory exception; retain it unchanged. Root npm currently has no overrides.

## Developer, build and docs groups

| Sites | Declared/source versus lock/observation | PROPOSED posture and trigger |
| --- | --- | --- |
| `rust-toolchain.toml`, workspace Rust floor | Rust1.95.0/MSRV1.95 | Retain;0/10 own environment match. No installed-ready claim. Reopen only with explicit toolchain/support need and full affected build proof. |
| `.nvmrc`, root/TS npm packageManager and TS engines | Node25.9.0; npm11.12.1; Node>=25 <26 | Already approved command-local10 preparation; retain current contract. Node26 type definitions do not change runtime range. |
| Python pyproject and tool guards | Ruff==0.15.17; Pyright==1.1.410; maturin>=1.9,<2 | Retain reviewed reproducibility pins and measured PEP-639 floor.10 resolves linked-worktree noneditable invocation. Any bump updates matched declarations/guards/tests after reviewed clean-checkout qualification; no incidental lint churn. |
| Root npm / lock / safe fixer | markdownlint-cli2 ^0.23.3→0.23.3, markdownlint0.41.1, markdown-it15.0.1; js-yaml5.4.1, smol-toml1.8.0, linkify-it6.1.0 | Retain until optional coherent candidate selected; preserve AST-neutral `npm run format:md`. No Prettier or unguarded --fix. Historical security comments are dated explanations, not current clearance. |
| TS npm / lock | TypeScript ^6.0.3→6.0.3; @types/node ^26.1.0→26.1.0 | Retain independently of Markdown group; reopen for need/advisory with tsc/public SDK/installed-addon proof. |
| Mermaid manifest / lock | @mermaid-js/mermaid-cli ^11.16.0→11.16.0; DOMPurify3.4.11; Puppeteer25.3.0/zod3.25.76 | Approved DOMPurify-only10 adaptation to3.4.17. Browser/zod candidate is distinct unselected group requiring real renderer qualification and actual config evidence. |
| `scripts/install-actionlint.sh` and lint guards | actionlint1.7.12 | Retain;7/10 coordinate invocation and pin guards. Reopen with applicable fix/support need and actionlint validation, not YAML safe_load. |
| `scripts/lib/shellcheck-version.sh`, installer | ShellCheck0.11.0, reviewed per-target archive hashes | Retain exact platform/checksum map, not blind latest. Reopen with pin source/provenance and supported installer/guard tests; no global tool replacement in1. |
| `scripts/lib/gitleaks-version.sh`, installer | github.com/zricethezav/gitleaks/v8@8.30.1 | Retain exact Go build metadata identity guard.7 owns cost/path proposal; an unrelated optimization cannot bypass scanner or substitute a version string. |
| `tools/docs/requirements.txt` | mkdocs==1.6.1 | Retain; reopen with docs need/advisory and public-doc lint/build/link proof. |
| `scripts/sbom-survey/pyproject.toml` | cyclonedx-python-lib[json-validation]>=8,<9; packageurl-python>=0.15,<1; packaging>=24,<26; semver>=3,<4; dev pytest>=8,<10; build setuptools>=68 | Retain isolated declared ranges; no lock here, installed/latest unknown.10 needs owned compatible tools to test behavior, not a package migration. Do not mix exporter packaging26.3 into this<26 environment. |
| `dev/tools/module-boundary-gate/Cargo.toml` / own lock | proc-macro2 1.0.106 range; syn2.0.117 range with parsing/full/visit | Retain tool's own lock; no migration selected. Reopen for Rust grammar/fix need with meaningful module-boundary positive/negative controls. |
| Experiment exact pins and own locks | dev experiments rusqlite=0.41.1; surrealdb=3.3.0 competitor driver; SDK-driver kernel patch above | Retain eval-only inputs. Evaluation owner reviews compatibility before reuse; no product security urgency inferred from competitor graph. Historical/fixture pins stay byte-frozen exclusions. |

All listed source locks preserve resolved transitive versions for reproducibility.
They are not a rationale to upgrade every node or a promise every node ships.
Open version/latest/license/yank research stays bounded as described in the
evidence register. Existing Python runtime `dependencies=[]` is distinct from
its dev/test extras and native Rust dependencies.

## Exporter exact group (all28 pins)

Edit site `dev/tools/onnx/export-requirements.txt`; dev-tooling, CPython3.12 Linux
x86_64 wheels, `--require-hashes --no-deps`, separate PyTorch CPU index. This is a
coupled reproducibility group, not Python SDK runtime dependencies. Retain each
pin/hash below unless selected separately; certifi/filelock/setuptools stay
investigate, transformers major stays postpone. Other pins reopen for applicable
advisory or explicit export need;10/next exporter owner assesses selected changes.
Do not regenerate model/golden hashes autonomously to make a candidate pass.
Current model SHA is
`c92689ecdefc35c3b533f68449cf1a50e7a1301736d8031372b0fa70e862c5a5`.

| Package | Exact pinned version | Retained wheel SHA-256 |
| --- | --- | --- |
| `certifi` | `2026.7.22` | `62f22742b58a1a33014a2b6b706588a8d7e2a88ae7bd1a6ebe8c992928483775` |
| `charset-normalizer` | `3.5.1` | `b9af956078716df40d985fb0dfeb2c2120c5ca92ba4ff4b388acfd01cdc14d08` |
| `filelock` | `3.32.3` | `7f0ca4bcc0e181c60dbbd8aa9ab5b120ebb99e4e064e83636340056f833a1f09` |
| `fsspec` | `2026.7.0` | `b57ddbafedfaef7018c1ecab32aa200a9d7ca26b77965f64e48b70061249d279` |
| `hf-xet` | `1.6.0` | `d62671bb130879cef0ee4c9ebe47a14af6c66ec53e6d84dc15936e5ffdfac82f` |
| `huggingface_hub` | `0.36.2` | `48f0c8eac16145dfce371e9d2d7772854a4f591bcb56c9cf548accf531d54270` |
| `idna` | `3.20` | `ab7ae7122974553370f0bdb919e1a960b2cd1bc1ef0276416d896db81c14582c` |
| `Jinja2` | `3.1.6` | `85ece4451f492d0c13c5dd7c13a64681a86afae63a5f347908daf103ce6d2f67` |
| `MarkupSafe` | `3.0.3` | `d6dd0be5b5b189d31db7cda48b91d7e0a9795f31430b7f271219ab30f1d3ac9d` |
| `ml_dtypes` | `0.5.4` | `9ad459e99793fa6e13bd5b7e6792c8f9190b4e5a1b45c63aba14a4d0a7f1d5ff` |
| `mpmath` | `1.3.0` | `a0b2b9fe80bbcd81a6647ff13108738cfb482d481d826cc0e02f5b35e5c88d2c` |
| `networkx` | `3.6.1` | `d47fbf302e7d9cbbb9e2555a0d267983d2aa476bac30e90dfbe5669bd57f3762` |
| `numpy` | `1.26.4` | `675d61ffbfa78604709862923189bad94014bef562cc35cf61d3a07bba02a7ed` |
| `onnx` | `1.23.0` | `f336004196a22fbdc16c62e7f26f20635af1826db6147c80ff3b4b8d428fc7ef` |
| `packaging` | `26.3` | `d7193f7c8e4e93f444fde0262bf90af30e16fa0ad0ad44cb553c87339b23cd1c` |
| `protobuf` | `7.36.2` | `89f23aa53c24553a2416fd4fd1ec06f74fa42b14b546d8883128813f775bbfd2` |
| `PyYAML` | `6.0.3` | `ba1cc08a7ccde2d2ec775841541641e4548226580ab850948cbfda66a1befcdc` |
| `regex` | `2026.9.10` | `2e67f8843f0e4b931f1fa860bf3bbe4134b714c0155cc5c7c0d7ea450230aae0` |
| `requests` | `2.34.2` | `2a0d60c172f83ac6ab31e4554906c0f3b3588d37b5cb939b1c061f4907e278e0` |
| `safetensors` | `0.8.0` | `fd6f3f93c9a0a7cc2788ee63fb763353d4bd2e89b0751bc78fcf7dda00bea774` |
| `setuptools` | `78.1.0` | `3e386e96793c8702ae83d17b853fb93d3e09ef82ec62722e61da5cd22376dcd8` |
| `sympy` | `1.14.0` | `e091cc3e99d2141a0ba2847328f5479b05d94a6635cb96148ccb3f34671bd8f5` |
| `tokenizers` | `0.19.1` | `7c9d5b6c0e7a1e979bec10ff960fae925e947aab95619a6fdb4c1d8ff3708ce3` |
| `torch` | `2.4.1+cpu` | `8800deef0026011d502c0c256cc4b67d002347f63c3a38cd8e45f1f445c61364` |
| `tqdm` | `4.70.1` | `c293e525e6fef9c20e8728fd4612df02a0aa31bb5fe91ecd93e123b1b7bffa73` |
| `transformers` | `4.44.2` | `1c02c65e7bfa5e52a634aff3da52138b583fc6f263c1f28d547dc144ba3d412d` |
| `typing_extensions` | `4.16.0` | `481caa481374e813c1b176ada14e97f1f67a4539ce9cfeb3f350d78d6370c2e8` |
| `urllib3` | `2.8.0` | `0cf3cae568d36aa9576b28dfb35f11328f1cb974ca7647d9475ebb86c75ac6e3` |

## Workflow action commit group

Every distinct tracked `uses: owner/repo@rev` below is a retained immutable pin;
16 distinct pairs were observed, including two download-artifact revisions.
Source comments/tag claims are not fresh remote commit provenance.7/10 own any
selected action update; target unknown/unselected. Trigger a concrete security,
runtime or delivery need, verify official commit/release identity, consuming job
permissions/platforms and relevant dry-run/installed-package path. A publish-only
job skipped by dry-run is unmeasured, not qualified. Never replace SHAs with tags
or publish merely to test the action. Narrow .29 first-tag routing is separately
approved10 scope, not authority to bump these pins.

| Action | Retained commit SHA |
| --- | --- |
| `PyO3/maturin-action` | `e83996d129638aa358a18fbd1dfb82f0b0fb5d3b` |
| `Swatinem/rust-cache` | `c19371144df3bb44fab255c43d04cbc2ab54d1c4` |
| `actions/cache` | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` |
| `actions/checkout` | `9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` |
| `actions/deploy-pages` | `d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e` |
| `actions/download-artifact` | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` |
| `actions/download-artifact` | `d3f86a106a0bac45b974a628896c90dbdf5c8093` |
| `actions/setup-node` | `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e` |
| `actions/setup-python` | `ece7cb06caefa5fff74198d8649806c4678c61a1` |
| `actions/upload-artifact` | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` |
| `actions/upload-pages-artifact` | `56afc609e74202658d3ffba0e8f6dda462b719fa` |
| `dorny/paths-filter` | `fbd0ab8f3e69293af611ebaee6363fc25e6d187d` |
| `dtolnay/rust-toolchain` | `fa04a1451ff1842e2626ccb99004d0195b455a88` |
| `pypa/gh-action-pypi-publish` | `cef221092ed1bacb1cc03d23a2d87d1d172e277b` |
| `rust-lang/crates-io-auth-action` | `c6f97d42243bad5fab37ca0427f495c86d5b1a18` |
| `softprops/action-gh-release` | `153bb8e04406b158c6c84fc1615b65b24149a1fe` |

[Selected candidate decisions](candidate-register.md) and
[qualification mapping](test-mapping.md) govern any promoted group. Exact-pin
review completeness is this enumerated group coverage plus source edit sites;
no comprehensive fresh registry/advisory/license assessment is claimed.
