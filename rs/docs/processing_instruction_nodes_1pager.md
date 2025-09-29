# Processing Instruction Node Support

## Context
- The Rust placeholder parser currently skips XML processing instructions (PIs) entirely, even though libxml2 models them as `xmlNode` records with `PiNode` type.
- Upcoming consumers (tree utilities, serialization) rely on PIs being present in the DOM with their target name and content preserved.

## Problem
- We lose information for inputs containing PIs, breaking compatibility with libxml2 and preventing downstream features (like stylesheet linking) from working in the Rust port.

## Goal
- Teach the Rust parser and document allocator to materialize PIs as proper nodes that mirror libxml2's behaviour (name = target, content = data, attached in document order).

## Non-Goals
- Full PI validation (e.g., forbidding `xml` targets outside the declaration) beyond what the placeholder parser already enforces.
- Streaming SAX callbacks for PIs.
- Advanced whitespace normalisation of PI data.

## Constraints
- Keep changes small and self-contained so they slot into the ongoing namespace/DOM scaffolding.
- Maintain libxml2-compatible memory layout and ownership semantics for any new allocations.
- Ensure new behaviour is covered by regression tests.

## Options Considered
1. **Materialise PIs as `PiNode` elements via dedicated allocator helper.**
   - **Pro:** Preserves structure faithfully and plugs into existing attachment logic without extra bookkeeping.
   - **Con:** Requires extending `XmlDocument` storage helpers.
   - **Risk:** Mismanaging string ownership could leak or dangle pointers.
2. **Represent PIs as comment nodes with encoded payload.**
   - **Pro:** Avoids new allocation paths.
   - **Con:** Breaks API expectations (wrong node type/content layout).
   - **Risk:** Downstream consumers misinterpret data, leading to subtle bugs.

## Decision
- Adopt option 1: add an allocator for PI nodes and update the parser to emit them, because it keeps semantics aligned with libxml2 with manageable implementation effort.

