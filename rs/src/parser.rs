use crate::doc::XmlDocument;
use crate::tree::xmlDoc;
use libc::{c_char, c_int};
use std::ptr;

// This is a placeholder for the parser context struct.
// It will be filled out as the implementation progresses.
#[allow(non_snake_case)]
#[repr(C)]
pub struct xmlParserCtxt {
    pub doc: *mut xmlDoc,
    pub wellFormed: c_int,
    pub options: c_int,
    pub input: *const c_char,
    pub input_size: c_int,
    pub base_url: *const c_char,
    pub encoding: *const c_char,
}

/// A placeholder implementation of xmlReadMemory.
///
/// This function is one of the main entry points for parsing an XML document
/// from a buffer in memory. The Rust port currently performs minimal
/// validation, creating a document shell that records the caller supplied
/// metadata.
///
/// # Safety
/// The caller must supply valid pointers for the input buffer and optional
/// strings (which may be null) following libxml2's C API contracts. The
/// returned pointer must be released with `xmlFreeDoc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadMemory(
    buffer: *const c_char,
    size: c_int,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if size < 0 || (size > 0 && buffer.is_null()) {
        return ptr::null_mut();
    }

    let ctxt = unsafe { xmlCreateMemoryParserCtxt(buffer, size) };
    if ctxt.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*ctxt).options = options;
        (*ctxt).base_url = url;
        (*ctxt).encoding = encoding;
    }

    let parse_rc = unsafe { xmlParseDocument(ctxt) };
    let well_formed = unsafe { (*ctxt).wellFormed };
    let doc_ptr = unsafe { (*ctxt).doc };
    unsafe {
        (*ctxt).doc = ptr::null_mut();
        xmlFreeParserCtxt(ctxt);
    }

    if parse_rc != 0 || well_formed == 0 || doc_ptr.is_null() {
        if let Some(doc) = unsafe { XmlDocument::from_raw(doc_ptr) } {
            drop(doc);
        }
        return ptr::null_mut();
    }

    doc_ptr
}

/// Create a parser context for parsing from an in-memory buffer.
///
/// # Safety
/// `buffer` must either be null (when `size` is zero) or point to at least
/// `size` bytes of readable memory. The returned context must eventually be
/// released with `xmlFreeParserCtxt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCreateMemoryParserCtxt(
    buffer: *const c_char,
    size: c_int,
) -> *mut xmlParserCtxt {
    if size < 0 || (size > 0 && buffer.is_null()) {
        return ptr::null_mut();
    }

    let ctxt = Box::new(xmlParserCtxt {
        doc: ptr::null_mut(),
        wellFormed: 0,
        options: 0,
        input: buffer,
        input_size: size,
        base_url: ptr::null(),
        encoding: ptr::null(),
    });

    Box::into_raw(ctxt)
}

/// Parse a document using the supplied parser context, synthesising a shell
/// document for downstream consumers.
///
/// # Safety
/// `ctxt` must be a valid pointer obtained from `xmlCreateMemoryParserCtxt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlParseDocument(ctxt: *mut xmlParserCtxt) -> c_int {
    if ctxt.is_null() {
        return -1;
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    if ctxt_ref.input_size > 0 && ctxt_ref.input.is_null() {
        ctxt_ref.wellFormed = 0;
        return -1;
    }

    let doc = unsafe { XmlDocument::new(ctxt_ref.options, ctxt_ref.base_url, ctxt_ref.encoding) };
    ctxt_ref.doc = doc.into_raw();
    ctxt_ref.wellFormed = 1;
    0
}

/// Release the resources held by a parser context.
///
/// # Safety
/// `ctxt` must be null or a pointer obtained from `xmlCreateMemoryParserCtxt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlFreeParserCtxt(ctxt: *mut xmlParserCtxt) {
    if ctxt.is_null() {
        return;
    }

    let mut ctxt = unsafe { Box::from_raw(ctxt) };
    if !ctxt.doc.is_null() {
        if let Some(doc) = unsafe { XmlDocument::from_raw(ctxt.doc) } {
            drop(doc);
        }
        ctxt.doc = ptr::null_mut();
    }
}
