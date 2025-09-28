use crate::tree::{xmlDoc, xmlElementType};
use libc::{c_char, c_int};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

const DEFAULT_VERSION: &[u8] = b"1.0\0";
const DEFAULT_ENCODING: &[u8] = b"UTF-8\0";

#[derive(Default)]
struct XmlDocExtras {
    version: Option<Box<[u8]>>,
    encoding: Option<Box<[u8]>>,
    url: Option<Box<[u8]>>,
}

impl XmlDocExtras {
    fn version_ptr(&self) -> *const u8 {
        self.version
            .as_deref()
            .map_or(DEFAULT_VERSION.as_ptr(), |bytes| bytes.as_ptr())
    }

    fn encoding_ptr(&self) -> *const u8 {
        self.encoding
            .as_deref()
            .map_or(DEFAULT_ENCODING.as_ptr(), |bytes| bytes.as_ptr())
    }

    fn url_ptr(&self) -> *const u8 {
        self.url
            .as_deref()
            .map_or(ptr::null(), |bytes| bytes.as_ptr())
    }
}

static DOC_EXTRAS: Lazy<Mutex<HashMap<usize, Box<XmlDocExtras>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Internal Rust-owned wrapper around `xmlDoc` providing RAII semantics.
///
/// This allows Rust code to manage the lifetime of documents safely while
/// still handing out raw pointers to the C API boundary. When the wrapper is
/// dropped the underlying allocation is reclaimed using Rust's allocator,
/// mirroring the behaviour of `xmlFreeDoc`.
pub struct XmlDocument {
    inner: NonNull<xmlDoc>,
    extras: Option<Box<XmlDocExtras>>,
}

impl XmlDocument {
    /// Allocate a new document populated with default metadata.
    ///
    /// # Safety
    /// `url` and `encoding` must either be null pointers or valid
    /// null-terminated strings that remain readable for the duration of this
    /// call.
    pub unsafe fn new(options: c_int, url: *const c_char, encoding: *const c_char) -> Self {
        let extras = XmlDocExtras {
            version: None,
            encoding: unsafe { duplicate_null_terminated(encoding as *const u8) },
            url: unsafe { duplicate_null_terminated(url as *const u8) },
        };
        Self::from_extras(options, extras)
    }

    /// Allocate a document honouring the provided XML version string.
    ///
    /// # Safety
    /// `version` must be either null or reference a valid null-terminated
    /// string that remains readable for the duration of this call.
    pub unsafe fn with_version(version: *const u8) -> Self {
        let extras = XmlDocExtras {
            version: unsafe { duplicate_null_terminated(version) },
            ..Default::default()
        };
        Self::from_extras(0, extras)
    }

    fn from_extras(options: c_int, extras: XmlDocExtras) -> Self {
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
            version: extras.version_ptr(),
            encoding: extras.encoding_ptr(),
            ids: ptr::null_mut(),
            refs: ptr::null_mut(),
            URL: extras.url_ptr(),
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

        XmlDocument {
            inner,
            extras: Some(Box::new(extras)),
        }
    }

    /// Borrow the underlying pointer for FFI exposure.
    pub fn as_ptr(&self) -> *mut xmlDoc {
        self.inner.as_ptr()
    }

    /// Transfer ownership of the allocation to the caller, preventing Drop
    /// from running.
    pub fn into_raw(mut self) -> *mut xmlDoc {
        let ptr = self.as_ptr();
        if let Some(extras) = self.extras.take() {
            register_extras(ptr, extras);
        }
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
        let inner = NonNull::new(doc)?;
        let extras = take_extras(doc);
        Some(XmlDocument { inner, extras })
    }
}

impl Drop for XmlDocument {
    fn drop(&mut self) {
        if self.extras.is_none()
            && let Some(extras) = take_extras(self.inner.as_ptr())
        {
            self.extras = Some(extras);
        }

        unsafe {
            drop(Box::from_raw(self.inner.as_ptr()));
        }

        if let Some(extras) = self.extras.take() {
            drop(extras);
        }
    }
}

fn register_extras(doc: *mut xmlDoc, extras: Box<XmlDocExtras>) {
    let mut map = DOC_EXTRAS.lock().expect("DOC_EXTRAS poisoned");
    map.insert(doc as usize, extras);
}

fn take_extras(doc: *mut xmlDoc) -> Option<Box<XmlDocExtras>> {
    let mut map = DOC_EXTRAS.lock().expect("DOC_EXTRAS poisoned");
    map.remove(&(doc as usize))
}

unsafe fn duplicate_null_terminated(ptr: *const u8) -> Option<Box<[u8]>> {
    if ptr.is_null() {
        return None;
    }

    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(ptr, len + 1);
        Some(slice.to_vec().into_boxed_slice())
    }
}

/// Allocate a new document populated with the provided XML version.
///
/// # Safety
/// `version` must be either null or a pointer to a valid null-terminated
/// string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlNewDoc(version: *const u8) -> *mut xmlDoc {
    let doc = unsafe { XmlDocument::with_version(version) };
    doc.into_raw()
}

/// Frees the memory allocated for an xmlDoc.
///
/// # Safety
/// The caller must ensure that `doc` either originated from one of the Rust
/// constructors and that it is not freed multiple times.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlFreeDoc(doc: *mut xmlDoc) {
    if let Some(doc) = unsafe { XmlDocument::from_raw(doc) } {
        drop(doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::ptr;

    fn reset_doc_extras() {
        DOC_EXTRAS.lock().expect("DOC_EXTRAS poisoned").clear();
    }

    #[test]
    fn xml_document_defaults_match_legacy_values() {
        reset_doc_extras();

        let doc = unsafe { XmlDocument::new(0, ptr::null(), ptr::null()) };
        let raw = doc.as_ptr();

        unsafe {
            let version = CStr::from_ptr((*raw).version as *const c_char);
            assert_eq!(version.to_str().unwrap(), "1.0");

            let encoding = CStr::from_ptr((*raw).encoding as *const c_char);
            assert_eq!(encoding.to_str().unwrap(), "UTF-8");

            assert!((*raw).URL.is_null());
        }
    }

    #[test]
    fn xml_document_round_trip_preserves_metadata() {
        reset_doc_extras();

        let url = CString::new("file:///tmp/example.xml").unwrap();
        let encoding = CString::new("ISO-8859-1").unwrap();

        let doc = unsafe { XmlDocument::new(42, url.as_ptr(), encoding.as_ptr()) };
        let raw = doc.into_raw();

        let doc = unsafe { XmlDocument::from_raw(raw) }.expect("document metadata");
        let c_doc = unsafe { &*doc.as_ptr() };

        assert_eq!(c_doc.parseFlags, 42);

        unsafe {
            let encoding = CStr::from_ptr(c_doc.encoding as *const c_char);
            assert_eq!(encoding.to_str().unwrap(), "ISO-8859-1");

            let url = CStr::from_ptr(c_doc.URL as *const c_char);
            assert_eq!(url.to_str().unwrap(), "file:///tmp/example.xml");
        }
    }

    #[test]
    fn xml_free_doc_clears_registered_metadata() {
        reset_doc_extras();

        let encoding = CString::new("UTF-16").unwrap();
        let doc = unsafe { XmlDocument::new(0, ptr::null(), encoding.as_ptr()) };
        let raw = doc.into_raw();

        unsafe {
            xmlFreeDoc(raw);
        }

        assert!(DOC_EXTRAS.lock().expect("DOC_EXTRAS poisoned").is_empty());
    }
}
