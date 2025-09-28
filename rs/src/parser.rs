use crate::doc::XmlDocument;
use crate::tree::xmlDoc;
use libc::{c_char, c_int};

// This is a placeholder for the parser context struct.
// It will be filled out as the implementation progresses.
#[allow(non_snake_case)]
#[repr(C)]
pub struct xmlParserCtxt {
    pub doc: *mut xmlDoc,
    pub wellFormed: c_int,
    pub options: c_int,
    // More fields will be added later.
}

/// A placeholder implementation of xmlReadMemory.
///
/// This function is one of the main entry points for parsing an XML document
/// from a buffer in memory. For now, it creates and returns a dummy document
/// to allow us to test the FFI linkage.
///
/// # Safety
/// The caller must supply valid pointers for the input buffer and optional
/// strings (which may be null) following libxml2's C API contracts. The
/// returned pointer must be released with `xmlFreeDoc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadMemory(
    _buffer: *const c_char,
    _size: c_int,
    _url: *const c_char,
    _encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    println!("[Rust] xmlReadMemory called (dummy implementation)");

    let doc = XmlDocument::new(options, _url);
    doc.into_raw()
}

/// Frees the memory allocated for an xmlDoc.
///
/// This function is essential for preventing memory leaks when the C test code
/// cleans up the documents created by `xmlReadMemory`.
///
/// # Safety
/// The caller must ensure that `doc` either originated from `xmlReadMemory`
/// (or another Rust-owned allocator) and that it is not freed multiple times.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlFreeDoc(doc: *mut xmlDoc) {
    if let Some(doc) = unsafe { XmlDocument::from_raw(doc) } {
        println!("[Rust] xmlFreeDoc called");
        drop(doc);
    }
}
