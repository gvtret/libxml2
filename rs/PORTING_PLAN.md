# Rust Porting Plan for libxml2

## Objectives
- Reimplement libxml2's core in Rust while preserving the public C API exposed through headers such as `parser.h` and `tree.h`, ensuring that downstream applications can link without modifications.
- Provide a drop-in replacement that mirrors existing behaviours (parsing, tree manipulation, validation, I/O) and integrates with the current build system.
- Incrementally replace C modules with Rust equivalents, maintaining ABI boundaries via FFI glue and shared data structures defined in Rust.

## Current Codebase Snapshot

### C architecture highlights
- **Streaming and tree parser** (`parser.c`): houses the SAX-driven core parser, progressive parsing entry points, and high-level helpers such as `xmlRead*` functions.
- **Tree manipulation** (`tree.c`): implements DOM-style node creation, mutation, and navigation APIs that operate on `xmlDoc`, `xmlNode`, and related structures.
- **Shared string dictionary** (`dict.c`): manages atomized strings, reference counting, and dictionary lifetimes relied upon by the parser.
- **I/O abstraction layer** (`xmlIO.c`): encapsulates file descriptors, custom protocol callbacks, and compression-aware streams used by the parser and serializers.
- **Validation and query layers** (`valid.c`, `xmlschemas.c`, `xmlregexp.c`, `xpath.c`): layered atop the parser/tree primitives; these will be later-phase ports once the foundations are stable.

### Existing Rust scaffolding
- `rs/src/tree.rs` defines Rust FFI representations of core structs (`xmlDoc`, `xmlNode`, `xmlAttr`, namespaces) matching the C layout.
- `rs/src/parser.rs` sketches FFI entry points such as `xmlReadMemory` and `xmlFreeDoc`, demonstrating ownership transfer between Rust and C.
- `rs/libxml2_rs.h` mirrors the Rust FFI types for C callers and is the temporary header until code generation (`cbindgen`) is integrated.

## Guiding Principles
1. **Preserve ABI/ABI**: Every Rust module must expose C-compatible symbols whose signatures remain byte-for-byte compatible with the legacy headers.
2. **Incremental rollout**: Use feature flags to compile either the C or Rust implementation, enabling side-by-side validation and fallback.
3. **Memory safety first**: Encapsulate raw pointers inside safe Rust abstractions as early as possible, leaving only the boundary layer unsafe.
4. **Test-driven parity**: Reuse existing regression suites and add Rust unit tests to validate behaviour across the transition.

## Porting Strategy

### Phase 1 – Data structures & glue
- Finalize `repr(C)` Rust definitions for `xmlDoc`, `xmlNode`, `xmlAttr`, dictionaries, buffers, and enums.
- Introduce shared Rust crates for reference-counted resources (dictionaries, input buffers) with safe wrappers that mirror current semantics.
- Provide FFI shims in C that delegate to the Rust implementations while retaining existing symbol names and linkage expectations.

### Phase 2 – Parser core
- Implement a Rust parser module that mirrors the control flow of `parser.c`, starting with well-formed document parsing (tokenization, tree construction).
- Route SAX callbacks through Rust traits/closures that populate DOM nodes via the `tree` abstractions.
- Support incremental parsing (`xmlCreatePushParserCtxt`, `xmlParseChunk`) to maintain streaming semantics.

### Phase 3 – Tree utilities & XPath foundation
- Port frequently used helpers from `tree.c` (node creation, namespace reconciliation, property access) to Rust.
- Re-implement XPath data model primitives in Rust to prepare for later migration of the query engine.
- Validate layout and behaviour with unit tests comparing Rust-created nodes to reference C structures.

### Phase 4 – Supporting subsystems
- Translate `dict.c` into a Rust intern pool using `Arc`/`Weak` for thread-safe reference counting.
- Re-implement `xmlIO.c` abstractions using Rust traits for input/output sources, including compression and custom protocol registration.
- Gradually port validation, schemas, and regexp engines, ensuring Rust modules can call back into any remaining C code until the migration is complete.

### Phase 5 – Build & packaging integration
- Extend Autotools, Meson, and CMake scripts to build the Rust crate and link it into the shared library.
- Generate canonical headers from Rust definitions using `cbindgen`, replacing `libxml2_rs.h` once stable.
- Provide configure-time switches (e.g., `--with-rust-core`) and CI matrix entries that compile both variants.

## Testing & Compatibility Plan
- Mirror existing C test suites (`runtest`, `runsuite`, fuzzers) against the Rust-backed library for regression coverage.
- Add Rust unit tests covering parser edge cases, memory management, and multi-threaded scenarios.
- Establish integration tests that compare parse trees produced by C vs. Rust implementations for representative XML inputs.

## Risk Mitigation & Tooling
- Introduce automated ABI checks (e.g., `cargo-c` or `abi-compliance-checker`) to detect signature/layout drift.
- Leverage sanitizers (`ASan`, `UBSan`) and Rust `miri` to validate memory safety during early hybrid phases.
- Maintain extensive documentation of FFI contracts to aid downstream users migrating custom extensions.

## Milestones
1. **Foundations complete** – Rust definitions and FFI glue compiled in CI; dummy parser delegates to existing C implementation.
2. **Minimal viable Rust parser** – Well-formed document parsing through Rust passes a subset of `runtest` cases.
3. **Feature parity** – Validation, XPath, and I/O subsystems achieve behaviour parity with C implementation.
4. **Performance tuning** – Optimize allocations and streaming to match or exceed C benchmarks; leverage profiling to identify regressions.
5. **C deprecation** – Retire redundant C modules once Rust reaches full compatibility, retaining legacy code behind build flags for transitional releases.

## Immediate Next Steps
- Audit `rs/src/parser.rs` for completeness against `parser.c` entry points and log missing functions.
- Prototype a Rust-owned document allocator with drop semantics mirroring `xmlFreeDoc`.
- Set up `cargo fmt`, `cargo clippy`, and CI integration to keep Rust code quality aligned with libxml2 standards.
