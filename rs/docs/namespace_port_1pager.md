# Namespace handling 1-Pager

## Context
- The Rust parser placeholder builds DOM trees using `XmlDocument` helpers and a simple streaming parser.
- Elements and attributes currently ignore XML namespace declarations, so `xmlNs` structures are never populated.
- Downstream callers rely on namespace metadata (`node->ns`, `node->nsDef`) to differentiate similarly named nodes.

## Problem
- Documents that declare default or prefixed namespaces lose that information in the Rust port.
- As a result, namespace-sensitive consumers would observe incorrect trees and regress when the Rust backend is enabled.

## Goal
- Teach the Rust placeholder parser to recognise namespace declaration attributes and populate `xmlNs` records.
- Ensure elements reference the correct namespace via `node->ns` and expose the declarations through `node->nsDef`.

## Non-Goals
- Full namespace scoping parity (e.g., namespace inheritance on attributes, namespace cleanup helpers).
- Implementing advanced XML namespace error handling or validation beyond the minimal checks required for correctness.
- Supporting entity-defined namespace URIs or DTD-driven namespace defaults.

## Constraints
- Keep changes local to the Rust parser scaffolding to avoid touching the large C sources for now.
- Reuse existing allocation helpers in `XmlDocument` so memory remains owned and freed safely.
- Maintain deterministic behaviour and keep functions within the stated complexity limits.
