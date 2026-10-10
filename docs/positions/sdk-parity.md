# SDK Parity

Python, TypeScript, and Rust (`fathomdb-sdk`) share the high-level application
operation surface in 0.8.27. The CLI remains a separate operator surface.

## Shared behavior

The governed-surface allowlist is shared by the Python and TypeScript
conformance suites. The Rust SDK operation map also checks its 44-operation
membership against that allowlist.

The SDKs provide:

- equivalent governed commands in idiomatic spelling, such as
  `read.get_many` / `read.getMany` and `erase_source` / `eraseSource`;
- equivalent error categories and structured payloads: Python uses
  `EngineError` subclasses, TypeScript uses `FathomDbError` subclasses, and
  Rust uses one `Error` with an `ErrorKind` for each category;
- equivalent result shapes, including typed `SearchHit.id`.

## Language conventions and boundaries

Python calls are synchronous; TypeScript returns Promises; Rust returns
synchronous `Result` values. Optional Rust arguments become option structs.
`drain` takes seconds in Python and milliseconds in TypeScript and Rust.
TypeScript accepts camelCase and snake_case for dual-spelled write keys.

The three SDKs share operation semantics and option defaults; parity does not
imply equal deployment history or performance on every host. Qualify the
chosen package and workload on the intended hardware.

The separate `fathomdb` crate is the lower-level Rust engine facade. Its
inherent methods and re-exported carriers have their own surface contract.
Use the [Rust installation guide](../install/rust.md) to distinguish the
lower-level facade from the application SDK.
