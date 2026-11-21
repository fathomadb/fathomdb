# FathomDB Inspector 0.2.0 local bundle

This bundle contains the Linux x86_64 executable, embedded UI, license, build
description and internal checksums. No Node, Python or development server is
needed to run it. The surrounding candidate/local distribution contains the tar,
outer `SHA256SUMS` and unsigned `candidate.json` source/build/support metadata.
A candidate becomes the local release only when90 promotes its exact bytes to
`dist/inspector/0.2.0/`. No registry release or human-study validation is claimed.

From the surrounding distribution directory:

```sh
sha256sum --check SHA256SUMS
mkdir inspector-install
tar -xzf fathomdb-inspector-0.2.0-linux-x86_64.tar.gz -C inspector-install
cd inspector-install/fathomdb-inspector-0.2.0-linux-x86_64
sha256sum --check SHA256SUMS
./fathomdb-inspector --help
./fathomdb-inspector --build-description
./fathomdb-inspector /absolute/path/to/existing.sqlite
```

Omit the database path to select it in the local UI. Close the application before
removing only its owned extraction folder/archive; preserve databases and caches.
Stop all external writers before attach. The selected Engine0.8.29 source commit
is `6edc1155f039f87cd0a37660cb27ae4373bbce71`, compiled schema 37. Normal owned
Engine startup/background/shutdown can change files. Missing/held lock, nonempty
initial WAL/journal, unsupported schema/layout/encoding and corrupt input refuse.
Preserve original files/sidecars; do not delete WAL or create a missing lock to
force admission. There is no migration, recovery or repair control.

## Exact qualified environment

Ubuntu 22.04/glibc 2.35 Linux x86_64 userland, kernel 7.0.0-43-generic/Landlock ABI 8,
Chromium 156.0.8078.4 in a graphical session. The compatible executable requires
at mostGLIBC 2.34. Cached CPU requires Landlock ABI >=3, so stock Ubuntu 22/kernel 5.15
is insufficient. This is not blanket newer-Linux/browser support. Allow four CPU
cores, 8 GiB available RAM and scratch at least twice the fixture size.

Shipping builds explicitly enable cached CPU and exclude fixture controls.
Native CPU queries need existing `BAAI/bge-small-en-v1.5` revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a` assets matching the build-description
hashes, dimension 384 and Engine identity `fathomdb-bge-small-en-v1.5`.
Inspector downloads/repairs nothing. Select model-free mode for independent
stored/text inspection when CPU assets are unavailable. No GPU/reranker chooser.

## Operation

Browse records by bounded stored title/name plus exact identity; inspect complete
document/tree, metadata and scalar values. Indexed text provides effective
property choices and nearby validation; original requests and separately timed
evidence remain distinguishable. Stored vector/L2 neighborhoods retain exact
origins. Directed graph branches retain explicit scope; source/dependency/history
navigation and Back preserve admitted original context. Pin two actual bounded
observations for typed membership and separate exact-origin correspondence;
methods retain their own ranks/values, without a common score scale.

Independent flags: `--no-show-inactive` hides otherwise retained history/inactive
records; `--show-erased` permits legitimately retained erasure content and narrow
selected export; `--show-payloads` permits telemetry payloads. Every applicable
gate must permit each read. Physically absent content is unavailable.
No general export, saved investigation, automatic rerun or shared snapshot.

Detach releases the owned session. Cancel expires it and requests termination;
UI acknowledgement is separate from native quiescence. Close the native window
normally to release private profile/bootstrap/worker resources. Read refusal,
partial, expired and evicted scope; explicit Refresh starts a new observation.
Preserve archive/checksums/build description when reporting an issue.
