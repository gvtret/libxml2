# Default Namespace Unbinding in the Rust Parser

## Context
- The Rust placeholder parser now attaches namespace declarations to elements and resolves prefixes using the bindings tracked on the element stack.
- Default namespace declarations (`xmlns="..."`) are currently treated like any other binding and always produce an `xmlNs` node with a non-null href.
- In libxml2's C implementation, declaring `xmlns=""` explicitly unbinds the default namespace for the scope of that element and its descendants.

## Problem
- The Rust port continues to associate elements with a namespace node even when the declaration uses an empty URI, leaving elements incorrectly namespaced instead of reverting to no namespace.
- This breaks compatibility with documents that intentionally reset the default namespace and diverges from libxml2's DOM tree semantics.

## Goal
- Match libxml2's behaviour for default namespace undeclarations by ensuring `xmlns=""` clears the namespace on the element and suppresses inheritance for descendants until the scope exits.

## Non-Goals
- Rejecting or warning about prefixed namespace undeclarations (`xmlns:prefix=""`), even though they are discouraged by the Namespaces in XML specification.
- Implementing namespace validation beyond recognising the built-in `xml` prefix.
- Reworking the overall namespace storage strategy in `XmlDocument` beyond what is required for empty bindings.

## Constraints
- Preserve the existing API surface of `XmlDocument` while allowing namespace allocations with null hrefs.
- Keep the placeholder parser deterministic and covered by regression tests.
- Avoid regressing other namespace scenarios already validated by the current unit tests.
