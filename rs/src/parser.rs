use crate::doc::{XmlDocument, xmlFreeDoc};
use crate::tree::xmlDoc;
use libc::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

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

#[allow(non_camel_case_types)]
pub type xmlInputReadCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, buffer: *mut c_char, len: c_int) -> c_int>;

#[allow(non_camel_case_types)]
pub type xmlInputCloseCallback = Option<unsafe extern "C" fn(context: *mut c_void) -> c_int>;

static PARSER_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

const XML_PARSE_RECOVER: c_int = 1 << 0;

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

/// Parse a buffer in recovery mode, mirroring `xmlRecoverMemory`.
///
/// # Safety
/// Delegates to `xmlReadMemory`; see that function for requirements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlRecoverMemory(
    buffer: *const c_char,
    size: c_int,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    unsafe { xmlReadMemory(buffer, size, url, encoding, options | XML_PARSE_RECOVER) }
}

/// Initialise the global parser state bookkeeping.
///
/// # Safety
/// Matches the C ABI contract: may be called from any thread without prior
/// initialisation. The function performs no memory unsafe operations.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlInitParser() {
    PARSER_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Tear down the global parser bookkeeping established by `xmlInitParser`.
///
/// # Safety
/// Safe to call multiple times and from any thread, mirroring the semantics of
/// the legacy C implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCleanupParser() {
    PARSER_INIT_COUNT.store(0, Ordering::SeqCst);
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

/// Parse a null-terminated buffer in recovery mode.
///
/// # Safety
/// Delegates to `xmlReadDoc`; see that function for requirements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlRecoverDoc(
    cur: *const u8,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    unsafe { xmlReadDoc(cur, url, encoding, options | XML_PARSE_RECOVER) }
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

/// Parse a document from disk using a SAX handler.
///
/// # Safety
/// `sax` and `user_data` may be null and are currently unused by the Rust
/// placeholder implementation. `filename` must be a valid null-terminated
/// string. Returns `0` on success and `-1` on failure, mirroring libxml2's C
/// API contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlSAXUserParseFile(
    sax: *mut c_void,
    user_data: *mut c_void,
    filename: *const c_char,
) -> c_int {
    let _ = (sax, user_data);

    let doc = unsafe { xmlReadFile(filename, ptr::null(), 0) };
    if doc.is_null() {
        return -1;
    }

    unsafe {
        xmlFreeDoc(doc);
    }

    0
}

/// Parse a document from a filesystem path in recovery mode.
///
/// # Safety
/// Delegates to `xmlReadFile`; see that function for requirements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlRecoverFile(
    filename: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    unsafe { xmlReadFile(filename, encoding, options | XML_PARSE_RECOVER) }
}

/// Parse an XML document from an existing file descriptor.
///
/// # Safety
/// The file descriptor must remain open for the duration of this call. It will
/// **not** be closed by this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadFd(
    fd: c_int,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if fd < 0 {
        return ptr::null_mut();
    }

    let ctxt = unsafe { xmlNewParserCtxt() };
    if ctxt.is_null() {
        return ptr::null_mut();
    }

    let doc = unsafe { xmlCtxtReadFd(ctxt, fd, url, encoding, options) };
    unsafe { xmlFreeParserCtxt(ctxt) };
    doc
}

/// Parse an in-memory document using a SAX handler.
///
/// # Safety
/// The placeholder parser validates the buffer using `xmlReadMemory` and does
/// not trigger callbacks on the provided SAX handler. `buffer` must either be
/// null (when `size` is zero) or reference a readable memory region of `size`
/// bytes. Returns `0` on success and `-1` otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlSAXUserParseMemory(
    sax: *mut c_void,
    user_data: *mut c_void,
    buffer: *const c_char,
    size: c_int,
) -> c_int {
    let _ = (sax, user_data);

    if size < 0 || (size > 0 && buffer.is_null()) {
        return -1;
    }

    let doc = unsafe { xmlReadMemory(buffer, size, ptr::null(), ptr::null(), 0) };
    if doc.is_null() {
        return -1;
    }

    unsafe {
        xmlFreeDoc(doc);
    }

    0
}

/// Parse a document from custom I/O callbacks, mirroring `xmlReadIO`.
///
/// # Safety
/// `ioread` must be a valid callback that reads from `ioctx` into the provided
/// buffer. `ioclose`, when non-null, is invoked after reading completes (even
/// on error). The returned document must be released with `xmlFreeDoc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlReadIO(
    ioread: xmlInputReadCallback,
    ioclose: xmlInputCloseCallback,
    ioctx: *mut c_void,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    let buffer = match unsafe { read_io_buffer(ioread, ioclose, ioctx) } {
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
            url,
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

/// Parse an XML document using the supplied context and file descriptor.
///
/// # Safety
/// The file descriptor must stay valid for the duration of the call and is not
/// closed when parsing completes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCtxtReadFd(
    ctxt: *mut xmlParserCtxt,
    fd: c_int,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if ctxt.is_null() || fd < 0 {
        return ptr::null_mut();
    }

    let buffer = match unsafe { read_fd_buffer(fd) } {
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
            url,
            encoding,
            options,
        )
    }
}

/// Parse an XML document into an existing context using custom I/O callbacks.
///
/// # Safety
/// `ctxt` must be a valid parser context and `ioread` must read from `ioctx`
/// according to libxml2's callback contracts. `ioclose`, when provided, is
/// invoked after reading completes (even on error).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCtxtReadIO(
    ctxt: *mut xmlParserCtxt,
    ioread: xmlInputReadCallback,
    ioclose: xmlInputCloseCallback,
    ioctx: *mut c_void,
    url: *const c_char,
    encoding: *const c_char,
    options: c_int,
) -> *mut xmlDoc {
    if ctxt.is_null() {
        return ptr::null_mut();
    }

    let buffer = match unsafe { read_io_buffer(ioread, ioclose, ioctx) } {
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
            url,
            encoding,
            options,
        )
    }
}

/// Allocate a fresh parser context initialised with default state.
///
/// # Safety
/// Returns a raw pointer that must be released with `xmlFreeParserCtxt`. The
/// caller is responsible for ensuring the context is not leaked or freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlNewParserCtxt() -> *mut xmlParserCtxt {
    let mut ctxt = Box::new(new_parser_context(ptr::null(), 0));
    if unsafe { xmlInitParserCtxt(&mut *ctxt) } != 0 {
        return ptr::null_mut();
    }

    Box::into_raw(ctxt)
}

/// Reset an existing parser context to its initial state.
///
/// # Safety
/// `ctxt` must be either null or a pointer obtained from one of the parser
/// context constructors. Passing any other pointer is undefined behaviour.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlInitParserCtxt(ctxt: *mut xmlParserCtxt) -> c_int {
    if ctxt.is_null() {
        return -1;
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    unsafe { reset_context_doc(ctxt_ref) };
    reset_context_state(ctxt_ref);

    0
}

/// Clear the transient parse state stored in a parser context.
///
/// # Safety
/// `ctxt` must be either null or a valid parser context pointer previously
/// returned by the Rust constructors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlClearParserCtxt(ctxt: *mut xmlParserCtxt) {
    if ctxt.is_null() {
        return;
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    unsafe { reset_context_doc(ctxt_ref) };
    reset_context_state(ctxt_ref);
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

    Box::into_raw(Box::new(new_parser_context(buffer, size)))
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

/// Create a parser context primed with a null-terminated in-memory document.
///
/// # Safety
/// `cur` must be a valid pointer to a null-terminated buffer that remains
/// accessible for the lifetime of the parser context unless replaced by other
/// parsing routines. The returned context must be freed with
/// `xmlFreeParserCtxt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCreateDocParserCtxt(cur: *const u8) -> *mut xmlParserCtxt {
    if cur.is_null() {
        return ptr::null_mut();
    }

    let len = unsafe { libc::strlen(cur as *const c_char) };
    if len > c_int::MAX as usize {
        return ptr::null_mut();
    }

    let ctxt = unsafe { xmlNewParserCtxt() };
    if ctxt.is_null() {
        return ptr::null_mut();
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    ctxt_ref.input = cur as *const c_char;
    ctxt_ref.input_size = len as c_int;

    ctxt
}

unsafe fn read_file_buffer(filename: *const c_char) -> Option<Vec<u8>> {
    if filename.is_null() {
        return None;
    }

    let cstr = unsafe { CStr::from_ptr(filename) };
    let path = pathbuf_from_cstr(cstr)?;
    fs::read(path).ok()
}

unsafe fn read_fd_buffer(fd: c_int) -> Option<Vec<u8>> {
    if fd < 0 {
        return None;
    }

    #[cfg(unix)]
    {
        use std::mem::ManuallyDrop;
        use std::os::unix::io::FromRawFd;

        let mut file = ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
        let mut data = Vec::new();
        if (*file).read_to_end(&mut data).is_err() {
            return None;
        }
        Some(data)
    }

    #[cfg(windows)]
    {
        use std::mem::ManuallyDrop;
        use std::os::windows::io::{FromRawHandle, RawHandle};

        let handle = unsafe { libc::_get_osfhandle(fd) };
        if handle == -1 {
            return None;
        }

        let mut file =
            ManuallyDrop::new(unsafe { std::fs::File::from_raw_handle(handle as RawHandle) });
        let mut data = Vec::new();
        if (*file).read_to_end(&mut data).is_err() {
            return None;
        }
        Some(data)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = fd;
        None
    }
}

unsafe fn read_io_buffer(
    ioread: xmlInputReadCallback,
    ioclose: xmlInputCloseCallback,
    ioctx: *mut c_void,
) -> Option<Vec<u8>> {
    const IO_CHUNK_SIZE: usize = 4096;

    let Some(read_cb) = ioread else {
        if let Some(close_cb) = ioclose {
            unsafe {
                close_cb(ioctx);
            }
        }
        return None;
    };

    let mut chunk = [0u8; IO_CHUNK_SIZE];
    let mut data = Vec::new();
    let mut had_error = false;

    loop {
        let read_rc = unsafe {
            read_cb(
                ioctx,
                chunk.as_mut_ptr() as *mut c_char,
                IO_CHUNK_SIZE as c_int,
            )
        };

        if read_rc == 0 {
            break;
        }

        if read_rc < 0 {
            had_error = true;
            break;
        }

        let read_usize = read_rc as usize;
        if read_usize > IO_CHUNK_SIZE {
            had_error = true;
            break;
        }

        data.extend_from_slice(&chunk[..read_usize]);
    }

    if let Some(close_cb) = ioclose {
        unsafe {
            close_cb(ioctx);
        }
    }

    if had_error { None } else { Some(data) }
}

fn new_parser_context(buffer: *const c_char, size: c_int) -> xmlParserCtxt {
    xmlParserCtxt {
        doc: ptr::null_mut(),
        wellFormed: 1,
        options: 0,
        input: buffer,
        input_size: size,
        base_url: ptr::null(),
        encoding: ptr::null(),
    }
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

fn reset_context_state(ctxt: &mut xmlParserCtxt) {
    ctxt.wellFormed = 1;
    ctxt.options = 0;
    ctxt.input = ptr::null();
    ctxt.input_size = 0;
    ctxt.base_url = ptr::null();
    ctxt.encoding = ptr::null();
}
