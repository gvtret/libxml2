# Built-in XML Namespace Support 1-Pager

## Context
The placeholder Rust parser now allocates and manages namespace declarations so
prefix bindings survive across FFI boundaries. However, XML reserves the `xml`
prefix for `http://www.w3.org/XML/1998/namespace`, and libxml2 exposes this
binding implicitly even when documents omit an explicit `xmlns:xml` declaration.
Today our Rust tree helpers and parser treat `xml` like any other prefix,
causing nodes such as `xml:lang` attributes to be left without a namespace.

## Problem
Ensure the Rust document allocator and parser recognise the implicit `xml`
prefix binding so tree consumers observe the same namespace pointers they would
receive from the legacy C parser.

## Goal
Provide built-in namespace support so elements and attributes using the `xml`
prefix automatically receive an `xmlNs` record pointing at the standard URI.

## Non-Goals
- Implement other reserved prefix behaviours (e.g., `xmlns`).
- Add validation for illegal uses of the `xml` prefix.
- Reconcile namespaces across arbitrary tree mutations.

## Constraints
- Namespace storage lives inside `XmlDocExtras`; clearing the tree must drop any
  allocations that depend on it.
- Behaviour must match libxml2 by handing out stable pointers for repeated uses
  within the same document.
- Changes must integrate with the existing placeholder parser without
  introducing global mutable state.

## Options Considered
1. **Doc-scoped cache**: Extend `XmlDocExtras` with an optional pointer storing a
   lazily-allocated `xmlNs` created on first use.
   - Pros: Reuses existing storage, guarantees per-document stability, avoids
     unsafe global sharing.
   - Cons: Requires bookkeeping when clearing the tree.
2. **Process-wide static `xmlNs`**: Allocate a single `xmlNs` via `Lazy` and hand
   out the same pointer to all documents.
   - Pros: Minimal per-document state.
   - Cons: The struct embeds a `context` pointer to the owning document; sharing
     violates libxml2 invariants and risks dangling pointers when documents are
     freed.
3. **On-demand allocations**: Create a fresh `xmlNs` each time the parser sees
   the `xml` prefix.
   - Pros: Simplest to wire in.
   - Cons: Breaks pointer equality expectations, leaks storage, and risks
     duplicates on the same element.

## Decision
Adopt **Option 1**. Caching the built-in namespace within `XmlDocExtras`
provides document-scoped stability without abusing global state, and it keeps
lifetime management straightforward when the tree is cleared.

## Follow-Up
- Reuse the cached pointer from other tree-building entry points once they are
  ported to Rust.
- Audit mutation helpers (e.g., attribute setters) to ensure they also surface
  the built-in namespace when needed.
