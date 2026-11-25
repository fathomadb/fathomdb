# Inspector availability and getting started

Inspector targets FathomDB 0.8.32 (schema 39, refusing schema 38 and older).
The `build-inspector` job
in `.github/workflows/release.yml` builds, checks ELF `GLIBC <= 2.39`, seals,
verifies, and attaches `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` to the FathomDB GitHub
Release. Local candidate builds can also be sealed, verified, and promoted
with `src/inspector/tools/local-release.py`. [Release notes](../releases/0.3.0.md)
explain the current boundary and verification steps.

The package builds against FathomDB Engine 0.8.32 source (compiled schema 39,
bundled SQLite, `TABLE_MANIFEST`-backed admission) and admits closed Schema 39
databases with all external writers stopped. Schema 38 (`0.8.31`) and older
databases refuse cleanly; Inspector
does not migrate older formats. It embeds the UI; Node, Python, GTK4/WebKitGTK,
and a development server are not application requirements. The qualified pairing
is Ubuntu 24.04 / glibc 2.39 Linux x86_64 userland (`D4 = B`), kernel
`7.0.0-43-generic` / Landlock ABI 8 and Chromium `156.0.8078.4`. Ubuntu 22.04 /
glibc 2.35 is not supported. Cached CPU requires Landlock ABI >= 3. This is an
exact pairing qualification, not all-Linux or all-Chromium support. Allow four
CPU cores, 8 GiB available RAM and scratch at least twice the inspected fixture
size.

From the GitHub Release download directory (or local distribution directory),
verify checksums and extract into a new owned directory:

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

Keep the original archive/checksums/build description for diagnosis. A graphical
session and existing supported Chromium are required. CPU semantic retrieval
also requires the exact already-cached BGE assets named by the build description;
model-free stored/text inspection remains usable without them. No download or
cache repair occurs. [Reference](../reference/index.md) records the pin.
To uninstall, close Inspector and delete only its owned extracted application
folder/archive. Your databases and model cache are separate and must be preserved.

From a source checkout with the documented Rust toolchain:

```sh
cargo build -p fathomdb-inspector --features cached-cpu --locked --offline
./target/debug/fathomdb-inspector --help
./target/debug/fathomdb-inspector --build-description
./target/debug/fathomdb-inspector /absolute/path/to/existing.sqlite
```

The offline build requires dependencies already cached. Launch requires Linux
x86_64, a graphical session and a supported installed Chromium browser. These
are development commands, not a published package or a compatibility claim for
all Linux distributions. The source README records detailed verification and
packaging prerequisites.

## Investigation workflow

1. Start the locally installed Inspector and open its browser interface.
2. Select an existing FathomDB database supported by that build.
3. Review the Engine-startup effects and meet the [attachment conditions](../operations/index.md).
4. Browse records and navigate the [text, vector, and graph views](../guides/index.md); pin two bounded observations to compare their identities and method-specific values.
5. Detach and close the owned Engine; normal startup/background/shutdown effects may have changed product files.

A database that cannot be attached is shown as unavailable with a reason;
it is not shown as empty. The Inspector does not create a new database to
replace a missing path, upgrade an old schema, repair files, or explicitly start a rebuild. Normal Engine-owned projection
work may run after admitted startup. Implemented views share these attachment boundaries; installed technical checks and historical unperformed human research
remain separate evidence.

See the [roadmap](../ROADMAP.md) for work beyond this initial scope.
