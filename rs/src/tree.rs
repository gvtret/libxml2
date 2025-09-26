// libxml2-rust/src/tree.rs

// This file contains the Rust definitions for the XML tree data structures,
// such as xmlDoc, xmlNode, and related enums.

use std::os::raw::{c_char, c_int, c_ushort, c_void};

// Corresponds to xmlElementType enum
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum xmlElementType {
    ElementNode = 1,
    AttributeNode = 2,
    TextNode = 3,
    CdataSectionNode = 4,
    EntityRefNode = 5,
    EntityNode = 6,
    PiNode = 7,
    CommentNode = 8,
    DocumentNode = 9,
    DocumentTypeNode = 10,
    DocumentFragNode = 11,
    NotationNode = 12,
    HtmlDocumentNode = 13,
    DtdNode = 14,
    ElementDecl = 15,
    AttributeDecl = 16,
    EntityDecl = 17,
    NamespaceDecl = 18,
    XincludeStart = 19,
    XincludeEnd = 20,
}

// Corresponds to xmlAttributeType enum
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum xmlAttributeType {
    AttributeCdata = 1,
    AttributeId,
    AttributeIdref,
    AttributeIdrefs,
    AttributeEntity,
    AttributeEntities,
    AttributeNmtoken,
    AttributeNmtokens,
    AttributeEnumeration,
    AttributeNotation,
}

// Redefining the structs with full fields now that they are declared.
#[repr(C)]
pub struct xmlNode {
    pub _private: *mut c_void,
    pub type_: xmlElementType,
    pub name: *const u8, // const xmlChar*
    pub children: *mut xmlNode,
    pub last: *mut xmlNode,
    pub parent: *mut xmlNode,
    pub next: *mut xmlNode,
    pub prev: *mut xmlNode,
    pub doc: *mut xmlDoc,
    pub ns: *mut xmlNs,
    pub content: *mut u8, // xmlChar*
    pub properties: *mut xmlAttr,
    pub nsDef: *mut xmlNs,
    pub psvi: *mut c_void,
    pub line: c_ushort,
    pub extra: c_ushort,
}

#[repr(C)]
pub struct xmlDoc {
    pub _private: *mut c_void,
    pub type_: xmlElementType,
    pub name: *mut c_char,
    pub children: *mut xmlNode,
    pub last: *mut xmlNode,
    pub parent: *mut xmlNode,
    pub next: *mut xmlNode,
    pub prev: *mut xmlNode,
    pub doc: *mut xmlDoc,

    pub compression: c_int,
    pub standalone: c_int,
    pub intSubset: *mut c_void, // xmlDtd
    pub extSubset: *mut c_void, // xmlDtd
    pub oldNs: *mut xmlNs,
    pub version: *const u8, // xmlChar
    pub encoding: *const u8, // xmlChar
    pub ids: *mut c_void,
    pub refs: *mut c_void,
    pub URL: *const u8, // xmlChar
    pub charset: c_int,
    pub dict: *mut c_void, // xmlDict
    pub psvi: *mut c_void,
    pub parseFlags: c_int,
    pub properties: c_int,
}

#[repr(C)]
pub struct xmlNs {
    pub next: *mut xmlNs,
    pub type_: xmlElementType, // xmlNsType is an alias for xmlElementType
    pub href: *const u8, // const xmlChar*
    pub prefix: *const u8, // const xmlChar*
    pub _private: *mut c_void,
    pub context: *mut xmlDoc,
}

#[repr(C)]
pub struct xmlAttr {
    pub _private: *mut c_void,
    pub type_: xmlElementType,
    pub name: *const u8, // const xmlChar*
    pub children: *mut xmlNode,
    pub last: *mut xmlNode,
    pub parent: *mut xmlNode,
    pub next: *mut xmlAttr,
    pub prev: *mut xmlAttr,
    pub doc: *mut xmlDoc,
    pub ns: *mut xmlNs,
    pub atype: xmlAttributeType,
    pub psvi: *mut c_void,
}