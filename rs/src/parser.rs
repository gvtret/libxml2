use crate::doc::XmlDocument;
use crate::tree::xmlDoc;
use libc::{c_char, c_int};
use std::ffi::CStr;
use std::fs;
use std::path::PathBuf;
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

    let doc = unsafe { xmlCtxtReadMemory(ctxt, buffer, size, url, encoding, options) };
    unsafe { xmlFreeParserCtxt(ctxt) };
    doc
}

/// Parse a full XML document provided as a null-terminated UTF-8 buffer.
///
/// # Safety
/// `cur` must point to a valid, null-terminated string containing the
/// serialized document. The returned pointer must be freed with
/// `xmlFreeDoc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadDoc(
    cur: *const u8,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if cur.is_null() {
        return ptr::null_mut();
    }

    let len = unsafe { libc::strlen(cur as *const c_char) };
    if len > c_int::MAX as usize {
        return ptr::null_mut();
    }

    unsafe { xmlReadMemory(cur as *const c_char, len as c_int, url, encoding, options) }
}

/// Parse a document from a filesystem path, loading the file into memory
/// before delegating to `xmlReadMemory`.
///
/// # Safety
/// `filename` must be a valid null-terminated string representing a
/// filesystem path that remains live for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadFile(
    filename: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    let buffer = match unsafe { read_file_buffer(filename) } {
        Some(buf) => buf,
        None => return ptr::null_mut(),
    };

    if buffer.len() > c_int::MAX as usize {
        return ptr::null_mut();
    }

    unsafe {
        xmlReadMemory(
            buffer.as_ptr() as *const c_char,
            buffer.len() as c_int,
            filename,
            encoding,
            options,
        )
    }
}

/// Parse a document held entirely in memory, mirroring libxml2's legacy API.
///
/// # Safety
/// Delegates to `xmlReadMemory`; see that function for requirements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlParseMemory(buffer: *const c_char, size: c_int) -> *mut xmlDoc {
    unsafe { xmlReadMemory(buffer, size, ptr::null(), ptr::null(), 0) }
}

/// Parse a document from a null-terminated buffer, returning a constructed
/// `xmlDoc` on success.
///
/// # Safety
/// `cur` must reference a valid, null-terminated string. The caller owns the
/// returned document and must release it with `xmlFreeDoc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlParseDoc(cur: *const u8) -> *mut xmlDoc {
    unsafe { xmlReadDoc(cur, ptr::null(), ptr::null(), 0) }
}

/// Parse a document directly from a file path using the default parsing
/// options.
///
/// # Safety
/// `filename` must be a valid null-terminated string representing a
/// filesystem path that remains live for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlParseFile(filename: *const c_char) -> *mut xmlDoc {
    unsafe { xmlReadFile(filename, ptr::null(), 0) }
}

/// Parse XML content into the provided parser context from an in-memory buffer.
///
/// # Safety
/// `ctxt` must be a valid pointer obtained from `xmlCreateMemoryParserCtxt` (or an
/// equivalent constructor once streaming support lands).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCtxtReadMemory(
    ctxt: *mut xmlParserCtxt,
    buffer: *const c_char,
    size: c_int,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if ctxt.is_null() || size < 0 || (size > 0 && buffer.is_null()) {
        return ptr::null_mut();
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    unsafe { reset_context_doc(ctxt_ref) };

    ctxt_ref.input = buffer;
    ctxt_ref.input_size = size;
    ctxt_ref.base_url = url;
    ctxt_ref.encoding = encoding;
    ctxt_ref.options = options;

    let parse_rc = unsafe { xmlParseDocument(ctxt) };
    let doc = unsafe { finalize_context_parse(ctxt_ref, parse_rc) };

    // The memory buffer is owned by the caller; clear our borrowed reference.
    ctxt_ref.input = ptr::null();
    ctxt_ref.input_size = 0;

    doc
}

/// Parse a null-terminated document string using an existing parser context.
///
/// # Safety
/// `cur` must point to a valid, null-terminated buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCtxtReadDoc(
    ctxt: *mut xmlParserCtxt,
    cur: *const u8,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if cur.is_null() {
        return ptr::null_mut();
    }

    let len = unsafe { libc::strlen(cur as *const c_char) };
    if len > c_int::MAX as usize {
        return ptr::null_mut();
    }

    unsafe {
        xmlCtxtReadMemory(
            ctxt,
            cur as *const c_char,
            len as c_int,
            url,
            encoding,
            options,
        )
    }
}

/// Load and parse a document from a file path using the supplied parser context.
///
/// # Safety
/// `filename` must be a valid null-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCtxtReadFile(
    ctxt: *mut xmlParserCtxt,
    filename: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    let buffer = match unsafe { read_file_buffer(filename) } {
        Some(buf) => buf,
        None => return ptr::null_mut(),
    };

    if buffer.len() > c_int::MAX as usize {
        return ptr::null_mut();
    }

    unsafe {
        xmlCtxtReadMemory(
            ctxt,
            buffer.as_ptr() as *const c_char,
            buffer.len() as c_int,
            filename,
            encoding,
            options,
        )
    }
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
    unsafe { reset_context_doc(&mut ctxt) };
}

unsafe fn read_file_buffer(filename: *const c_char) -> Option<Vec<u8>> {
    if filename.is_null() {
        return None;
    }

    let cstr = unsafe { CStr::from_ptr(filename) };
    let path = pathbuf_from_cstr(cstr)?;
    fs::read(path).ok()
}

fn pathbuf_from_cstr(cstr: &CStr) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        Some(PathBuf::from(OsString::from_vec(cstr.to_bytes().to_vec())))
    }

    #[cfg(not(unix))]
    {
        let string = cstr.to_str().ok()?;
        Some(PathBuf::from(string))
    }
}

unsafe fn reset_context_doc(ctxt: &mut xmlParserCtxt) {
    if ctxt.doc.is_null() {
        return;
    }

    if let Some(doc) = unsafe { XmlDocument::from_raw(ctxt.doc) } {
        drop(doc);
    }
    ctxt.doc = ptr::null_mut();
}

unsafe fn finalize_context_parse(ctxt: &mut xmlParserCtxt, parse_rc: c_int) -> *mut xmlDoc {
    let well_formed = ctxt.wellFormed;
    let doc_ptr = ctxt.doc;
    ctxt.doc = ptr::null_mut();

    if parse_rc != 0 || well_formed == 0 || doc_ptr.is_null() {
        if let Some(doc) = unsafe { XmlDocument::from_raw(doc_ptr) } {
            drop(doc);
        }
        return ptr::null_mut();
    }

    doc_ptr
}
