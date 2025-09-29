use crate::tree::{xmlAttr, xmlAttributeType, xmlDoc, xmlElementType, xmlNode, xmlNs};
use libc::{c_char, c_int};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

const DEFAULT_VERSION: &[u8] = b"1.0\0";
const DEFAULT_ENCODING: &[u8] = b"UTF-8\0";
const XML_NAMESPACE_PREFIX: &[u8] = b"xml";
const XML_NAMESPACE_URI: &[u8] = b"http://www.w3.org/XML/1998/namespace";

#[allow(clippy::vec_box)]
#[derive(Default)]
struct XmlDocExtras {
    version: Option<Box<[u8]>>,
    encoding: Option<Box<[u8]>>,
    url: Option<Box<[u8]>>,
    node_storage: Vec<Box<xmlNode>>,
    attr_storage: Vec<Box<xmlAttr>>,
    string_storage: Vec<Box<[u8]>>,
    ns_storage: Vec<Box<xmlNs>>,
    xml_namespace: Option<NonNull<xmlNs>>,
}

unsafe impl Send for XmlDocExtras {}

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

    fn alloc_string(&mut self, data: &[u8]) -> *mut u8 {
        let mut owned = Vec::with_capacity(data.len() + 1);
        owned.extend_from_slice(data);
        owned.push(0);
        let boxed = owned.into_boxed_slice();
        let ptr = boxed.as_ptr() as *mut u8;
        self.string_storage.push(boxed);
        ptr
    }

    fn alloc_const_string(&mut self, data: &[u8]) -> *const u8 {
        self.alloc_string(data) as *const u8
    }

    fn alloc_node(&mut self, node: xmlNode) -> *mut xmlNode {
        let mut boxed = Box::new(node);
        let ptr = boxed.as_mut() as *mut xmlNode;
        self.node_storage.push(boxed);
        ptr
    }

    fn alloc_attr(&mut self, attr: xmlAttr) -> *mut xmlAttr {
        let mut boxed = Box::new(attr);
        let ptr = boxed.as_mut() as *mut xmlAttr;
        self.attr_storage.push(boxed);
        ptr
    }

    fn alloc_ns(&mut self, ns: xmlNs) -> *mut xmlNs {
        let mut boxed = Box::new(ns);
        let ptr = boxed.as_mut() as *mut xmlNs;
        self.ns_storage.push(boxed);
        ptr
    }

    fn clear_tree_storage(&mut self) {
        self.node_storage.clear();
        self.attr_storage.clear();
        self.string_storage.clear();
        self.ns_storage.clear();
        self.xml_namespace = None;
    }

    fn ensure_xml_namespace(&mut self, doc_ptr: *mut xmlDoc) -> *mut xmlNs {
        if let Some(ns) = self.xml_namespace {
            return ns.as_ptr();
        }

        let href_ptr = self.alloc_const_string(XML_NAMESPACE_URI);
        let prefix_ptr = self.alloc_const_string(XML_NAMESPACE_PREFIX);
        let ns_ptr = self.alloc_ns(xmlNs {
            next: ptr::null_mut(),
            type_: xmlElementType::NamespaceDecl,
            href: href_ptr,
            prefix: prefix_ptr,
            _private: ptr::null_mut(),
            context: doc_ptr,
        });
        self.xml_namespace = NonNull::new(ns_ptr);
        ns_ptr
    }

    fn set_version(&mut self, version: &[u8]) -> *const u8 {
        self.version = Some(to_c_string(version));
        self.version_ptr()
    }

    fn set_encoding(&mut self, encoding: &[u8]) -> *const u8 {
        self.encoding = Some(to_c_string(encoding));
        self.encoding_ptr()
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
            encoding: unsafe { duplicate_null_terminated(encoding as *const u8) },
            url: unsafe { duplicate_null_terminated(url as *const u8) },
            ..Default::default()
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

    pub fn as_mut_ptr(&mut self) -> *mut xmlDoc {
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

    fn extras_mut(&mut self) -> &mut XmlDocExtras {
        self.extras
            .as_deref_mut()
            .expect("XmlDocument extras must be present")
    }

    pub fn clear_tree(&mut self) {
        let doc_ptr = self.as_mut_ptr();
        unsafe {
            (*doc_ptr).children = ptr::null_mut();
            (*doc_ptr).last = ptr::null_mut();
        }
        self.extras_mut().clear_tree_storage();
    }

    pub fn set_version_bytes(&mut self, version: &[u8]) {
        let ptr = self.extras_mut().set_version(version);
        unsafe {
            (*self.inner.as_ptr()).version = ptr;
        }
    }

    pub fn set_encoding_bytes(&mut self, encoding: &[u8]) {
        let ptr = self.extras_mut().set_encoding(encoding);
        unsafe {
            (*self.inner.as_ptr()).encoding = ptr;
        }
    }

    pub fn alloc_element(&mut self, name: &[u8]) -> *mut xmlNode {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        let name_ptr = extras.alloc_const_string(name);
        extras.alloc_node(xmlNode {
            _private: ptr::null_mut(),
            type_: xmlElementType::ElementNode,
            name: name_ptr,
            children: ptr::null_mut(),
            last: ptr::null_mut(),
            parent: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            doc: doc_ptr,
            ns: ptr::null_mut(),
            content: ptr::null_mut(),
            properties: ptr::null_mut(),
            nsDef: ptr::null_mut(),
            psvi: ptr::null_mut(),
            line: 0,
            extra: 0,
        })
    }

    pub fn alloc_text_node(&mut self, content: &[u8], node_type: xmlElementType) -> *mut xmlNode {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        let content_ptr = extras.alloc_string(content);
        extras.alloc_node(xmlNode {
            _private: ptr::null_mut(),
            type_: node_type,
            name: ptr::null(),
            children: ptr::null_mut(),
            last: ptr::null_mut(),
            parent: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            doc: doc_ptr,
            ns: ptr::null_mut(),
            content: content_ptr,
            properties: ptr::null_mut(),
            nsDef: ptr::null_mut(),
            psvi: ptr::null_mut(),
            line: 0,
            extra: 0,
        })
    }

    pub fn alloc_processing_instruction(&mut self, target: &[u8], content: &[u8]) -> *mut xmlNode {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        let name_ptr = extras.alloc_const_string(target);
        let content_ptr = if content.is_empty() {
            ptr::null_mut()
        } else {
            extras.alloc_string(content)
        };

        extras.alloc_node(xmlNode {
            _private: ptr::null_mut(),
            type_: xmlElementType::PiNode,
            name: name_ptr,
            children: ptr::null_mut(),
            last: ptr::null_mut(),
            parent: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            doc: doc_ptr,
            ns: ptr::null_mut(),
            content: content_ptr,
            properties: ptr::null_mut(),
            nsDef: ptr::null_mut(),
            psvi: ptr::null_mut(),
            line: 0,
            extra: 0,
        })
    }

    pub fn alloc_attribute(&mut self, name: &[u8]) -> *mut xmlAttr {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        let name_ptr = extras.alloc_const_string(name);
        extras.alloc_attr(xmlAttr {
            _private: ptr::null_mut(),
            type_: xmlElementType::AttributeNode,
            name: name_ptr,
            children: ptr::null_mut(),
            last: ptr::null_mut(),
            parent: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            doc: doc_ptr,
            ns: ptr::null_mut(),
            atype: xmlAttributeType::AttributeCdata,
            psvi: ptr::null_mut(),
        })
    }

    pub fn alloc_namespace(&mut self, href: Option<&[u8]>, prefix: Option<&[u8]>) -> *mut xmlNs {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        let href_ptr = match href {
            Some(value) if !value.is_empty() => extras.alloc_const_string(value),
            Some(_) | None => ptr::null(),
        };
        let prefix_ptr = prefix
            .filter(|value| !value.is_empty())
            .map(|value| extras.alloc_const_string(value))
            .unwrap_or(ptr::null());
        extras.alloc_ns(xmlNs {
            next: ptr::null_mut(),
            type_: xmlElementType::NamespaceDecl,
            href: href_ptr,
            prefix: prefix_ptr,
            _private: ptr::null_mut(),
            context: doc_ptr,
        })
    }

    pub fn ensure_xml_namespace(&mut self) -> *mut xmlNs {
        let doc_ptr = self.inner.as_ptr();
        let extras = self.extras_mut();
        extras.ensure_xml_namespace(doc_ptr)
    }

    /// # Safety
    /// `parent` and `child` must either be null or pointers produced by the
    /// Rust allocation helpers in this module. The pointers must remain valid
    /// for the duration of the call and are re-linked according to libxml2's
    /// tree invariants.
    pub unsafe fn attach_child(&mut self, parent: Option<*mut xmlNode>, child: *mut xmlNode) {
        unsafe {
            (*child).next = ptr::null_mut();
            (*child).prev = ptr::null_mut();
            (*child).parent = parent.unwrap_or(ptr::null_mut());

            match parent {
                Some(parent_ptr) => {
                    if (*parent_ptr).children.is_null() {
                        (*parent_ptr).children = child;
                        (*parent_ptr).last = child;
                    } else {
                        let mut last = (*parent_ptr).last;
                        if last.is_null() {
                            last = (*parent_ptr).children;
                            while !(*last).next.is_null() {
                                last = (*last).next;
                            }
                        }
                        (*child).prev = last;
                        (*last).next = child;
                        (*parent_ptr).last = child;
                    }
                }
                None => {
                    let doc_ptr = self.inner.as_ptr();
                    if (*doc_ptr).children.is_null() {
                        (*doc_ptr).children = child;
                        (*doc_ptr).last = child;
                    } else {
                        let mut last = (*doc_ptr).last;
                        if last.is_null() {
                            last = (*doc_ptr).children;
                            while !(*last).next.is_null() {
                                last = (*last).next;
                            }
                        }
                        (*child).prev = last;
                        (*last).next = child;
                        (*doc_ptr).last = child;
                    }
                }
            }
        }
    }

    /// # Safety
    /// `element` and `attr` must originate from the Rust helpers in this
    /// module. The attribute pointer is linked into the element's property
    /// list without additional validation.
    pub unsafe fn append_attribute(&mut self, element: *mut xmlNode, attr: *mut xmlAttr) {
        unsafe {
            (*attr).parent = element;
            (*attr).next = ptr::null_mut();
            (*attr).prev = ptr::null_mut();
            if (*element).properties.is_null() {
                (*element).properties = attr;
            } else {
                let mut current = (*element).properties;
                while !(*current).next.is_null() {
                    current = (*current).next;
                }
                (*current).next = attr;
                (*attr).prev = current;
            }
        }
    }

    /// # Safety
    /// `element` and `ns` must be null or pointers allocated through this
    /// module. The namespace list on the element is extended without
    /// additional validation.
    pub unsafe fn append_namespace(&mut self, element: *mut xmlNode, ns: *mut xmlNs) {
        if element.is_null() || ns.is_null() {
            return;
        }

        unsafe {
            (*ns).next = ptr::null_mut();

            if (*element).nsDef.is_null() {
                (*element).nsDef = ns;
            } else {
                let mut current = (*element).nsDef;
                while !(*current).next.is_null() {
                    current = (*current).next;
                }
                (*current).next = ns;
            }
        }
    }

    /// # Safety
    /// `element` must be null or a pointer allocated through this module.
    /// When `ns` is `None` the element namespace is cleared.
    pub unsafe fn set_node_namespace(&mut self, element: *mut xmlNode, ns: Option<*mut xmlNs>) {
        if element.is_null() {
            return;
        }

        unsafe {
            (*element).ns = ns.unwrap_or(ptr::null_mut());
        }
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
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, len + 1) };
    Some(slice.to_vec().into_boxed_slice())
}

fn to_c_string(data: &[u8]) -> Box<[u8]> {
    let mut owned = Vec::with_capacity(data.len() + 1);
    owned.extend_from_slice(data);
    owned.push(0);
    owned.into_boxed_slice()
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
