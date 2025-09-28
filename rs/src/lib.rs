//! Rust scaffolding that re-implements portions of libxml2 behind the
//! canonical C API surface.
//!
//! The modules exported here are intentionally minimal and are expected to be
//! accessed primarily through `libxml.h` by C callers. Tests and internal
//! helpers can import the modules directly to validate invariants during the
//! ongoing Rust port.

pub mod doc;
pub mod parser;
pub mod tree;
