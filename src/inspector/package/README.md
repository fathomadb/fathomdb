# FathomDB Inspector 0.3.0 bundle

This bundle contains the Linux x86_64 executable, embedded UI, license, build
description and internal checksums. No Node, Python or development server is
needed to run it. Release builds are delivered as GitHub Release assets
(`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` from the `build-inspector`
release job) and can also be sealed, verified, and promoted locally via
`local-release.py`. No human-study validation is claimed.

From the download or local distribution directory:

```sh
sha256sum --check fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256
# (or `sha256sum --check SHA256SUMS` from a local candidate directory)
mkdir inspector-install
tar -xzf fathomdb-inspector-0.3.0-linux-x86_64.tar.gz -C inspector-install
cd inspector-install/fathomdb-inspector-0.3.0-linux-x86_64
sha256sum --check SHA256SUMS
./fathomdb-inspector --help
./fathomdb-inspector --build-description
./fathomdb-inspector /absolute/path/to/existing.sqlite
```

Omit the database path to select it in the local UI. Close the application before
removing only its owned extraction folder/archive; preserve databases and caches.
Stop all external writers before attach. Inspector builds against
FathomDB `0.8.32` Engine source (compiled schema `39`, bundled SQLite,
`TABLE_MANIFEST`-backed table/column admission and 8-column `vec0` `scope TEXT`
shape validation) and admits closed Schema `39` databases; Schema `38`
(`0.8.31`) and older or non-conforming layouts refuse cleanly without mutating
the file. Normal owned Engine startup/background/shutdown can change files after
admission.
Missing/held lock, nonempty initial WAL/journal, unsupported schema/layout/encoding
and corrupt input refuse. Preserve original files/sidecars; do not delete WAL or
create a missing lock to force admission. There is no migration, recovery or
repair control.

## Exact qualified environment

Ubuntu 24.04 / glibc 2.39 Linux x86_64 userland (`D4 = B`, bundled SQLite,
zero GTK4/WebKitGTK or Docker requirements), kernel 7.0.0-43-generic /
Landlock ABI 8, Chromium 156.0.8078.4 in a graphical session. The native
executable requires at most GLIBC 2.39 (Ubuntu 22.04 / glibc 2.35 is not
supported). Cached CPU requires Landlock ABI >= 3. This is not blanket
newer-Linux/browser support. Allow four CPU cores, 8 GiB available RAM and
scratch at least twice the fixture size.

Shipping builds explicitly enable cached CPU and exclude fixture controls.
Native CPU queries need existing `BAAI/bge-small-en-v1.5` revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a` assets matching the build-description
hashes, dimension 384 and Engine identity `fathomdb-bge-small-en-v1.5`.
Inspector downloads/repairs nothing. Select model-free mode for independent
stored/text inspection when CPU assets are unavailable. No GPU/reranker chooser.

## Operation

Browse records by bounded stored title/name plus exact identity; inspect complete
`body` documents (plain text or JSON), outside-`body` typed node and edge
`properties` (expandable tree and exact copy preserving 64-bit integer lexemes
outside `2^53`, `-0`, empty strings, and `SQL NULL`), active-`scope` switching
and scope filters across inventory, list, lexical, serving, and graph views
(uncluttered on single-scope `""` stores; strictly subordinate to `D-01`
nondisclosure so scopes whose only rows are hidden by `--no-show-inactive` or
default erased hiding never appear in `catalog().scopes` or dropdowns), metadata,
and scalar values. Indexed text provides effective property choices and nearby
validation; original requests and separately timed evidence remain
distinguishable, and body/property lexical results remain record-keyed rather
than merged across equal bodies. Stored vector/L2 neighborhoods retain exact
origins within the seed's scope. Directed graph branches support both current
and historical `as_of` snapshots (`as_of_write_cursor` or `as_of_epoch_ms`,
mutually exclusive) across stored-adjacency and native engine traversals while
retaining explicit scope; source/dependency/history navigation and Back preserve
admitted original context. Pin two actual bounded observations (including
distinct `as_of` graph snapshots) for typed membership and separate exact-origin
correspondence; methods retain their own ranks/values, without a common score
scale.

Independent flags: `--no-show-inactive` hides otherwise retained history/inactive
records (including in historical `as_of` graph queries); `--show-erased` permits
legitimately retained erasure content and narrow selected export;
`--show-payloads` permits telemetry payloads. Every applicable gate must permit
each read across `body`, `properties` (and `properties_bytes`/storage metadata),
and historical graph targets/edges/origins. Physically absent content is
unavailable. No general export, saved investigation, automatic rerun or shared
snapshot.

Detach releases the owned session. Cancel expires it and requests termination;
UI acknowledgement is separate from native quiescence. Close the native window
normally to release private profile/bootstrap/worker resources. Read refusal,
partial, expired and evicted scope; explicit Refresh starts a new observation.
Preserve archive/checksums/build description when reporting an issue.
