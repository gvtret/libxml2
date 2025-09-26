use crate::tree::{xmlDoc, xmlElementType, xmlNode};
use libc::{c_char, c_int, c_void};

// This is a placeholder for the parser context struct.
// It will be filled out as the implementation progresses.
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadMemory(
    _buffer: *const c_char,
    _size: c_int,
    _url: *const c_char,
    _encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    println!("[Rust] xmlReadMemory called (dummy implementation)");

    // Allocate a new xmlDoc on the heap using Box for proper memory management.
    let doc = Box::new(xmlDoc {
        _private: std::ptr::null_mut(),
        type_: xmlElementType::DocumentNode,
        name: std::ptr::null_mut(),
        children: std::ptr::null_mut(),
        last: std::ptr::null_mut(),
        parent: std::ptr::null_mut(),
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
        doc: std::ptr::null_mut(), // This will be set to point to itself.
        compression: 0,
        standalone: 1,
        intSubset: std::ptr::null_mut(),
        extSubset: std::ptr::null_mut(),
        oldNs: std::ptr::null_mut(),
        version: "1.0".as_ptr(),
        encoding: "UTF-8".as_ptr(),
        ids: std::ptr::null_mut(),
        refs: std::ptr::null_mut(),
        URL: _url as *const u8,
        charset: 0,
        dict: std::ptr::null_mut(),
        psvi: std::ptr::null_mut(),
        parseFlags: options,
        properties: 0,
    });

    // Convert the Box into a raw pointer to pass to C.
    // The C code is now responsible for this memory.
    let doc_ptr = Box::into_raw(doc);
    (*doc_ptr).doc = doc_ptr;

    doc_ptr
}

/// Frees the memory allocated for an xmlDoc.
///
/// This function is essential for preventing memory leaks when the C test code
/// cleans up the documents created by `xmlReadMemory`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlFreeDoc(doc: *mut xmlDoc) {
    if !doc.is_null() {
        println!("[Rust] xmlFreeDoc called");
        // Re-Box the raw pointer to allow Rust to deallocate it properly.
        let _ = Box::from_raw(doc);
    }
}