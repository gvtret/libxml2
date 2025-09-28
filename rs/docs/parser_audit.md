# `parser.c` API audit

This document captures the current coverage of libxml2's public parser entry points in the Rust port. It will be expanded as we
progress through Phase 1 of the porting plan.

## Summary
- :white_check_mark: `xmlReadMemory` is stubbed out in Rust to exercise the FFI surface.
- :white_check_mark: `xmlFreeDoc` frees the dummy document allocation through the new RAII wrapper.
- :white_check_mark: Parser context lifecycle helpers (`xmlCreateMemoryParserCtxt`, `xmlParseDocument`, `xmlFreeParserCtxt`, `xmlNewParserCtxt`, `xmlInitParserCtxt`, `xmlClearParserCtxt`, `xmlCreateDocParserCtxt`) are now stubbed to retain metadata and manage document ownership in Rust.
- :white_check_mark: Global parser initialisation and teardown (`xmlInitParser`, `xmlCleanupParser`) are stubbed to maintain compatibility with the C entry points.
- :white_check_mark: `xmlReadDoc`, `xmlParseDoc`, and `xmlParseMemory` reuse the Rust `xmlReadMemory` stub for in-memory parsing.
- :white_check_mark: `xmlReadFile`/`xmlParseFile` now load the target file and reuse the in-memory stub to provide consistent behaviour.
- :white_check_mark: `xmlReadFd` and `xmlCtxtReadFd` read from existing descriptors without taking ownership and delegate to the in-memory flow.
- :white_check_mark: `xmlReadIO` and `xmlCtxtReadIO` bridge custom I/O callbacks through the in-memory placeholder parser while ensuring callbacks are closed.
- :white_check_mark: `xmlCtxtReadMemory`, `xmlCtxtReadDoc`, and `xmlCtxtReadFile` reuse the placeholder parser with an existing context.
- :white_check_mark: `xmlRecoverMemory`, `xmlRecoverDoc`, and `xmlRecoverFile` reuse the read helpers with recovery parsing enabled.
- :x: All other parser-facing functions still call into the legacy C implementation and need Rust shims.

## Entry points

| Function | Rust status | Notes |
| --- | --- | --- |
| `xmlReadMemory` | ✅ Stubbed | Returns placeholder document via `XmlDocument`. |
| `xmlReadFile` | ✅ Stubbed | Reads the file then calls `xmlReadMemory`. |
| `xmlReadFd` | ✅ Stubbed | Reads from descriptor without closing and reuses `xmlReadMemory`. |
| `xmlReadDoc` | ✅ Stubbed | Delegates to `xmlReadMemory`. |
| `xmlReadIO` | ✅ Stubbed | Reads callback data into memory before parsing. |
| `xmlCtxtReadMemory` | ✅ Stubbed | Delegates to the Rust placeholder parser. |
| `xmlCtxtReadIO` | ✅ Stubbed | Reuses callback bridge with existing context. |
| `xmlCtxtReadFd` | ✅ Stubbed | Loads descriptor contents then routes through `xmlCtxtReadMemory`. |
| `xmlCtxtReadFile` | ✅ Stubbed | Loads from disk then routes through `xmlCtxtReadMemory`. |
| `xmlParseDoc` | ✅ Stubbed | Reuses `xmlReadDoc` stub. |
| `xmlParseMemory` | ✅ Stubbed | Routes to `xmlReadMemory`. |
| `xmlParseFile` | ✅ Stubbed | Delegates to `xmlReadFile` with default options. |
| `xmlSAXUserParseFile` | ❌ Missing | Requires SAX handler bridging. |
| `xmlSAXUserParseMemory` | ❌ Missing | " |
| `xmlCreatePushParserCtxt` | ❌ Missing | Needs streaming parser implementation. |
| `xmlParseChunk` | ❌ Missing | Streaming support pending. |
| `xmlStopParser` | ❌ Missing | Depends on parser state machine. |
| `xmlResumeParser` | ❌ Missing | " |
| `xmlClearParserCtxt` | ✅ Stubbed | Drops any owned document and resets parser metadata. |
| `xmlInitParserCtxt` | ✅ Stubbed | Resets the lightweight Rust context state. |
| `xmlCreateDocParserCtxt` | ✅ Stubbed | Wraps `xmlNewParserCtxt` and records the caller's buffer metadata. |
| `xmlCreateMemoryParserCtxt` | ✅ Stubbed | Records caller metadata without performing real parsing. |
| `xmlParseDocument` | ✅ Stubbed | Synthesises a shell document and marks the context as well-formed. |
| `xmlFreeParserCtxt` | ✅ Stubbed | Drops the Rust-owned document if present. |
| `xmlInitParser` | ✅ Stubbed | Tracks init calls to maintain observable side effects. |
| `xmlCleanupParser` | ✅ Stubbed | Clears the init bookkeeping state. |
| `xmlNewParserCtxt` | ✅ Stubbed | Allocates a lightweight context shell. |
| `xmlRecoverMemory` | ✅ Stubbed | Delegates to `xmlReadMemory` with recovery flag. |
| `xmlRecoverDoc` | ✅ Stubbed | Reuses `xmlReadDoc` and recovery options. |
| `xmlRecoverFile` | ✅ Stubbed | Calls `xmlReadFile` with recovery enabled. |

## Next steps
- Introduce a thin abstraction layer that allows C entry points to toggle between Rust and legacy implementations.
- Prioritise fleshing out the remaining non-streaming helpers before addressing streaming and SAX integration.
