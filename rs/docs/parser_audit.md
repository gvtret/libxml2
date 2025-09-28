# `parser.c` API audit

This document captures the current coverage of libxml2's public parser entry points in the Rust port. It will be expanded as we
progress through Phase 1 of the porting plan.

## Summary
- :white_check_mark: `xmlReadMemory` is stubbed out in Rust to exercise the FFI surface.
- :white_check_mark: `xmlFreeDoc` frees the dummy document allocation through the new RAII wrapper.
- :white_check_mark: Parser context lifecycle helpers (`xmlCreateMemoryParserCtxt`, `xmlParseDocument`, `xmlFreeParserCtxt`) are now stubbed to retain metadata and manage document ownership in Rust.
- :white_check_mark: `xmlReadDoc`, `xmlParseDoc`, and `xmlParseMemory` reuse the Rust `xmlReadMemory` stub for in-memory parsing.
- :white_check_mark: `xmlReadFile` and `xmlParseFile` now load the target file and reuse the in-memory stub to provide consistent behaviour.
- :x: All other parser-facing functions still call into the legacy C implementation and need Rust shims.

## Entry points

| Function | Rust status | Notes |
| --- | --- | --- |
| `xmlReadMemory` | ✅ Stubbed | Returns placeholder document via `XmlDocument`. |
| `xmlReadFile` | ✅ Stubbed | Reads the file then calls `xmlReadMemory`. |
| `xmlReadFd` | ❌ Missing | Requires Rust I/O abstraction (Phase 4 dependency). |
| `xmlReadDoc` | ✅ Stubbed | Delegates to `xmlReadMemory`. |
| `xmlReadIO` | ❌ Missing | Blocked on Rust `xmlIO` port. |
| `xmlCtxtReadMemory` | ❌ Missing | Depends on parser context modelling. |
| `xmlCtxtReadIO` | ❌ Missing | Requires context + I/O integration. |
| `xmlCtxtReadFd` | ❌ Missing | " |
| `xmlCtxtReadFile` | ❌ Missing | " |
| `xmlParseDoc` | ✅ Stubbed | Reuses `xmlReadDoc` stub. |
| `xmlParseMemory` | ✅ Stubbed | Routes to `xmlReadMemory`. |
| `xmlParseFile` | ✅ Stubbed | Delegates to `xmlReadFile` with default options. |
| `xmlSAXUserParseFile` | ❌ Missing | Requires SAX handler bridging. |
| `xmlSAXUserParseMemory` | ❌ Missing | " |
| `xmlCreatePushParserCtxt` | ❌ Missing | Needs streaming parser implementation. |
| `xmlParseChunk` | ❌ Missing | Streaming support pending. |
| `xmlStopParser` | ❌ Missing | Depends on parser state machine. |
| `xmlResumeParser` | ❌ Missing | " |
| `xmlClearParserCtxt` | ❌ Missing | Context lifecycle currently unimplemented. |
| `xmlCreateMemoryParserCtxt` | ✅ Stubbed | Records caller metadata without performing real parsing. |
| `xmlParseDocument` | ✅ Stubbed | Synthesises a shell document and marks the context as well-formed. |
| `xmlFreeParserCtxt` | ✅ Stubbed | Drops the Rust-owned document if present. |
| `xmlInitParser` | ❌ Missing | Needs global init shared with dictionaries. |
| `xmlCleanupParser` | ❌ Missing | Mirror init/cleanup in Rust. |
| `xmlCreateDocParserCtxt` | ❌ Missing | Depends on context modelling. |
| `xmlNewParserCtxt` | ❌ Missing | " |
| `xmlRecoverMemory` | ❌ Missing | Hooks into recovery mode. |
| `xmlRecoverDoc` | ❌ Missing | " |
| `xmlRecoverFile` | ❌ Missing | " |

## Next steps
- Flesh out `xmlParserCtxt` representation in Rust so that context-based entry points can be stubbed.
- Introduce a thin abstraction layer that allows C entry points to toggle between Rust and legacy implementations.
- Prioritise fleshing out the remaining non-streaming helpers (`xmlReadFile`, `xmlParseFile`, etc.) to build confidence before
addressing streaming and SAX integration.
