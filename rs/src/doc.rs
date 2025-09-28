use crate::tree::{xmlDoc, xmlElementType};
use libc::{c_char, c_int};
use std::ptr::{self, NonNull};

/// Internal Rust-owned wrapper around `xmlDoc` providing RAII semantics.
///
/// This allows Rust code to manage the lifetime of documents safely while
/// still handing out raw pointers to the C API boundary. When the wrapper is
/// dropped the underlying allocation is reclaimed using Rust's allocator,
/// mirroring the behaviour of `xmlFreeDoc`.
pub struct XmlDocument {
    inner: NonNull<xmlDoc>,
}

impl XmlDocument {
    const VERSION: &'static [u8] = b"1.0\0";
    const ENCODING: &'static [u8] = b"UTF-8\0";

    /// Allocate a new document populated with default metadata.
    pub fn new(options: c_int, url: *const c_char) -> Self {
        // Allocate the structure with the same default values that the legacy
        // C implementation relies on when creating an empty document.
        let doc = Box::new(xmlDoc {
            _private: ptr::null_mut(),
            type_: xmlElementType::DocumentNode,
            name: ptr::null_mut(),
            children: ptr::null_mut(),
            last: ptr::null_mut(),
            parent: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            doc: ptr::null_mut(),
            compression: 0,
            standalone: 1,
            intSubset: ptr::null_mut(),
            extSubset: ptr::null_mut(),
            oldNs: ptr::null_mut(),
            version: Self::VERSION.as_ptr(),
            encoding: Self::ENCODING.as_ptr(),
            ids: ptr::null_mut(),
            refs: ptr::null_mut(),
            URL: url as *const u8,
            charset: 0,
            dict: ptr::null_mut(),
            psvi: ptr::null_mut(),
            parseFlags: options,
            properties: 0,
        });

        // Convert the Box into a raw pointer so that the document can be
        // shared across the FFI boundary. The `doc` self-pointer is populated
        // afterwards to mirror libxml2's invariants.
        let mut inner = NonNull::new(Box::into_raw(doc)).expect("xmlDoc allocation");
        unsafe {
            inner.as_mut().doc = inner.as_ptr();
        }

        XmlDocument { inner }
    }

    /// Borrow the underlying pointer for FFI exposure.
    pub fn as_ptr(&self) -> *mut xmlDoc {
        self.inner.as_ptr()
    }

    /// Transfer ownership of the allocation to the caller, preventing Drop
    /// from running.
    pub fn into_raw(self) -> *mut xmlDoc {
        let ptr = self.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reconstitute the RAII wrapper from a raw pointer previously produced by
    /// `into_raw` or handed to us over FFI.
    ///
    /// # Safety
    /// The caller must ensure that `doc` was allocated by `XmlDocument::new`
    /// (or an equivalent constructor that uses Rust's allocator) and has not
    /// already been freed or wrapped in another `XmlDocument` instance.
    pub unsafe fn from_raw(doc: *mut xmlDoc) -> Option<Self> {
        NonNull::new(doc).map(|inner| XmlDocument { inner })
    }
}

impl Drop for XmlDocument {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.inner.as_ptr()));
        }
    }
}
