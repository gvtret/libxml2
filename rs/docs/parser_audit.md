# `parser.c` API audit

This document captures the current coverage of libxml2's public parser entry points in the Rust port. It will be expanded as we
progress through Phase 1 of the porting plan.

## Summary
- :white_check_mark: `xmlReadMemory` is stubbed out in Rust to exercise the FFI surface.
- :white_check_mark: `xmlFreeDoc` frees the dummy document allocation through the new RAII wrapper.
- :x: All other parser-facing functions still call into the legacy C implementation and need Rust shims.

## Entry points

| Function | Rust status | Notes |
| --- | --- | --- |
| `xmlReadMemory` | ✅ Stubbed | Returns placeholder document via `XmlDocument`. |
| `xmlReadFile` | ❌ Missing | Should delegate to memory/path helper once available. |
| `xmlReadFd` | ❌ Missing | Requires Rust I/O abstraction (Phase 4 dependency). |
| `xmlReadDoc` | ❌ Missing | Thin wrapper over memory parsing. |
| `xmlReadIO` | ❌ Missing | Blocked on Rust `xmlIO` port. |
| `xmlCtxtReadMemory` | ❌ Missing | Depends on parser context modelling. |
| `xmlCtxtReadIO` | ❌ Missing | Requires context + I/O integration. |
| `xmlCtxtReadFd` | ❌ Missing | " |
| `xmlCtxtReadFile` | ❌ Missing | " |
| `xmlParseDoc` | ❌ Missing | Should call into Rust parser core. |
| `xmlParseMemory` | ❌ Missing | " |
| `xmlParseFile` | ❌ Missing | " |
| `xmlSAXUserParseFile` | ❌ Missing | Requires SAX handler bridging. |
| `xmlSAXUserParseMemory` | ❌ Missing | " |
| `xmlCreatePushParserCtxt` | ❌ Missing | Needs streaming parser implementation. |
| `xmlParseChunk` | ❌ Missing | Streaming support pending. |
| `xmlStopParser` | ❌ Missing | Depends on parser state machine. |
| `xmlResumeParser` | ❌ Missing | " |
| `xmlClearParserCtxt` | ❌ Missing | Context lifecycle currently unimplemented. |
| `xmlFreeParserCtxt` | ❌ Missing | Will drop Rust-owned context wrapper. |
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
- Prioritise implementing the non-streaming functions (`xmlReadMemory`, `xmlReadDoc`, `xmlParseDoc`) to build confidence before
addressing streaming and SAX integration.
