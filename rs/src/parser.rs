use crate::doc::{XmlDocument, xmlFreeDoc};
use crate::tree::{xmlDoc, xmlElementType, xmlNode};
use libc::{c_char, c_int, c_void};
use once_cell::sync::Lazy;
use std::char;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs;
use std::io::Read;
use std::mem;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::sync::Mutex;
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
    pub sax: *mut xmlSAXHandler,
    pub user_data: *mut c_void,
    pub disableSAX: c_int,
}

#[allow(non_camel_case_types)]
pub type xmlInputReadCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, buffer: *mut c_char, len: c_int) -> c_int>;

#[allow(non_camel_case_types)]
pub type xmlInputCloseCallback = Option<unsafe extern "C" fn(context: *mut c_void) -> c_int>;

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct xmlSAXHandler {
    _private: *mut c_void,
}

static PARSER_INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

const XML_PARSE_RECOVER: c_int = 1 << 0;

#[derive(Default)]
struct PushParserState {
    buffer: Vec<u8>,
    stopped: bool,
    terminated: bool,
}

static PUSH_PARSER_STATES: Lazy<Mutex<HashMap<usize, PushParserState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Parse an XML document stored entirely in memory and return a fully
/// populated `xmlDoc` tree.
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

    let bytes: &[u8] = if size == 0 {
        if buffer.is_null() {
            &[]
        } else {
            let len = unsafe { libc::strlen(buffer) } as usize;
            unsafe { slice::from_raw_parts(buffer as *const u8, len) }
        }
    } else {
        unsafe { slice::from_raw_parts(buffer as *const u8, size as usize) }
    };

    match parse_document_from_bytes(bytes, options, url, encoding) {
        Ok(doc) => doc.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Create a push-style parser context capable of consuming data incrementally.
///
/// # Safety
/// `chunk` must either be null (when `size` is zero) or reference a readable
/// memory region with at least `size` bytes. The returned context must be
/// released with `xmlFreeParserCtxt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlCreatePushParserCtxt(
    sax: *mut xmlSAXHandler,
    user_data: *mut c_void,
    chunk: *const c_char,
    size: c_int,
    filename: *const c_char,
) -> *mut xmlParserCtxt {
    if size < 0 || (size > 0 && chunk.is_null()) {
        return ptr::null_mut();
    }

    let ctxt = unsafe { xmlNewParserCtxt() };
    if ctxt.is_null() {
        return ptr::null_mut();
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    ctxt_ref.sax = sax;
    ctxt_ref.user_data = user_data;
    ctxt_ref.base_url = filename;

    let mut state = PushParserState::default();
    if size > 0 {
        let slice = unsafe { std::slice::from_raw_parts(chunk as *const u8, size as usize) };
        state.buffer.extend_from_slice(slice);
    }

    register_push_state(ctxt, state);

    ctxt
}

/// Feed data into an existing push-style parser context.
///
/// # Safety
/// `chunk` must be either null (when `size` is zero) or point to at least
/// `size` readable bytes. Set `terminate` to a non-zero value once no more data
/// will be supplied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlParseChunk(
    ctxt: *mut xmlParserCtxt,
    chunk: *const c_char,
    size: c_int,
    terminate: c_int,
) -> c_int {
    if ctxt.is_null() || size < 0 || (size > 0 && chunk.is_null()) {
        return -1;
    }

    let key = ctxt as usize;
    let (maybe_buffer, was_stopped) = {
        let mut map = PUSH_PARSER_STATES
            .lock()
            .expect("push parser state poisoned");
        let state = match map.get_mut(&key) {
            Some(state) => state,
            None => {
                return -1;
            }
        };

        if state.stopped {
            (None, true)
        } else {
            if size > 0 {
                let slice =
                    unsafe { std::slice::from_raw_parts(chunk as *const u8, size as usize) };
                state.buffer.extend_from_slice(slice);
            }

            if terminate != 0 {
                state.terminated = true;
                (Some(mem::take(&mut state.buffer)), false)
            } else {
                (None, false)
            }
        }
    };

    if was_stopped {
        return -1;
    }

    if let Some(buffer) = maybe_buffer {
        if buffer.len() > c_int::MAX as usize {
            drop(buffer);
            clear_push_state(ctxt);
            return -1;
        }

        let len = buffer.len() as c_int;
        let doc = unsafe {
            xmlCtxtReadMemory(
                ctxt,
                buffer.as_ptr() as *const c_char,
                len,
                (*ctxt).base_url,
                (*ctxt).encoding,
                (*ctxt).options,
            )
        };

        clear_push_state(ctxt);

        if doc.is_null() { -1 } else { 0 }
    } else {
        0
    }
}

/// Halt any further parsing activity on the supplied parser context.
///
/// # Safety
/// `ctxt` must be either null or a valid parser context pointer obtained from
/// one of the Rust constructors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlStopParser(ctxt: *mut xmlParserCtxt) {
    if ctxt.is_null() {
        return;
    }

    let key = ctxt as usize;
    if let Ok(mut map) = PUSH_PARSER_STATES.lock()
        && let Some(state) = map.get_mut(&key)
    {
        state.stopped = true;
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    ctxt_ref.disableSAX = 2;
    ctxt_ref.wellFormed = 0;
}

/// Resume parsing on a push-style parser context that was previously stopped.
///
/// # Safety
/// `ctxt` must be either null or a valid pointer obtained from one of the Rust
/// constructors. Returns `0` on success and `-1` if the parser cannot be
/// resumed (for example, if it has already been terminated).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmlResumeParser(ctxt: *mut xmlParserCtxt) -> c_int {
    if ctxt.is_null() {
        return -1;
    }

    let key = ctxt as usize;
    {
        let mut map = match PUSH_PARSER_STATES.lock() {
            Ok(map) => map,
            Err(_) => return -1,
        };

        match map.get_mut(&key) {
            Some(state) if !state.terminated => {
                state.stopped = false;
            }
            Some(_) => return -1,
            None => return -1,
        }
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    ctxt_ref.disableSAX = 0;
    if ctxt_ref.wellFormed == 0 {
        ctxt_ref.wellFormed = 1;
    }

    0
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
    clear_push_state(ctxt);
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

#[unsafe(no_mangle)]
/// Parse the buffer registered on the supplied context and produce a document
/// tree when the input is well-formed.
///
/// # Safety
/// `ctxt` must be a valid pointer obtained from the parser-context
/// constructors. The context's `input` and `input_size` fields must describe a
/// readable memory region that remains accessible for the duration of this
/// call.
pub unsafe extern "C" fn xmlParseDocument(ctxt: *mut xmlParserCtxt) -> c_int {
    if ctxt.is_null() {
        return -1;
    }

    let ctxt_ref = unsafe { &mut *ctxt };
    unsafe { reset_context_doc(ctxt_ref) };

    if ctxt_ref.input_size < 0 {
        ctxt_ref.wellFormed = 0;
        return -1;
    }

    let bytes: &[u8] = if ctxt_ref.input_size == 0 {
        if ctxt_ref.input.is_null() {
            &[]
        } else {
            let len = unsafe { libc::strlen(ctxt_ref.input) } as usize;
            unsafe { slice::from_raw_parts(ctxt_ref.input as *const u8, len) }
        }
    } else {
        if ctxt_ref.input.is_null() {
            ctxt_ref.wellFormed = 0;
            return -1;
        }
        unsafe { slice::from_raw_parts(ctxt_ref.input as *const u8, ctxt_ref.input_size as usize) }
    };

    match parse_document_from_bytes(
        bytes,
        ctxt_ref.options,
        ctxt_ref.base_url,
        ctxt_ref.encoding,
    ) {
        Ok(doc) => {
            ctxt_ref.doc = doc.into_raw();
            ctxt_ref.wellFormed = 1;
            0
        }
        Err(_) => {
            ctxt_ref.wellFormed = 0;
            -1
        }
    }
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
    let ctxt_ptr: *mut xmlParserCtxt = &mut *ctxt;
    clear_push_state(ctxt_ptr);
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

fn parse_document_from_bytes(
    bytes: &[u8],
    options: c_int,
    url: *const c_char,
    encoding: *const c_char,
) -> Result<XmlDocument, ()> {
    let mut doc = unsafe { XmlDocument::new(options, url, encoding) };
    SimpleParser::parse_into(&mut doc, bytes)?;
    Ok(doc)
}

struct SimpleParser<'a> {
    data: &'a [u8],
    pos: usize,
    doc: &'a mut XmlDocument,
    stack: Vec<*mut xmlNode>,
    root_count: usize,
}

type AttributeRecord = (Vec<u8>, Vec<u8>);

impl<'a> SimpleParser<'a> {
    fn parse_into(doc: &'a mut XmlDocument, bytes: &'a [u8]) -> Result<(), ()> {
        let data = strip_utf8_bom(bytes);
        let mut parser = SimpleParser {
            data,
            pos: 0,
            doc,
            stack: Vec::new(),
            root_count: 0,
        };

        parser.doc.clear_tree();
        parser.skip_whitespace();
        parser.parse_xml_declaration()?;

        while parser.pos < parser.data.len() {
            parser.skip_whitespace();
            if parser.pos >= parser.data.len() {
                break;
            }

            if parser.starts_with(b"<!--") {
                parser.parse_comment()?;
            } else if parser.starts_with(b"<?") {
                parser.parse_processing_instruction()?;
            } else if parser.starts_with(b"</") {
                parser.parse_end_element()?;
            } else if parser.data[parser.pos] == b'<' {
                parser.parse_start_element()?;
            } else {
                parser.parse_text_node()?;
            }
        }

        if parser.root_count == 1 && parser.stack.is_empty() {
            Ok(())
        } else {
            Err(())
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn starts_with(&self, pattern: &[u8]) -> bool {
        self.data[self.pos..].starts_with(pattern)
    }

    fn parse_xml_declaration(&mut self) -> Result<(), ()> {
        if !self.starts_with(b"<?xml") {
            return Ok(());
        }

        self.pos += 5;
        loop {
            self.skip_whitespace();
            if self.starts_with(b"?>") {
                self.pos += 2;
                break;
            }

            let name = self.parse_name()?;
            self.skip_whitespace();
            self.expect_char(b'=')?;
            self.skip_whitespace();
            let quote = self.next_byte()?;
            if quote != b'"' && quote != b'\'' {
                return Err(());
            }
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.data.len() && self.data[self.pos] != quote {
                self.pos += 1;
            }
            if self.pos >= self.data.len() {
                return Err(());
            }
            let value = &self.data[start..self.pos];
            self.pos += 1;

            match name.as_slice() {
                b"version" => self.doc.set_version_bytes(value),
                b"encoding" => self.doc.set_encoding_bytes(value),
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_start_element(&mut self) -> Result<(), ()> {
        self.expect_char(b'<')?;
        let name = self.parse_name()?;
        let attrs = self.parse_attributes()?;

        let empty = if self.consume_char(b'/') {
            self.expect_char(b'>')?;
            true
        } else {
            self.expect_char(b'>')?;
            false
        };

        let node = self.doc.alloc_element(&name);
        self.attach_attributes(node, attrs)?;

        let parent = self.stack.last().copied();
        if parent.is_none() {
            self.root_count += 1;
            if self.root_count > 1 {
                return Err(());
            }
        }
        unsafe {
            self.doc.attach_child(parent, node);
        }

        if !empty {
            self.stack.push(node);
        }

        Ok(())
    }

    fn parse_end_element(&mut self) -> Result<(), ()> {
        self.expect_sequence(b"</")?;
        let name = self.parse_name()?;
        self.skip_whitespace();
        self.expect_char(b'>')?;

        let node = self.stack.pop().ok_or(())?;
        let node_name = node_name_bytes(node);
        if node_name != name {
            return Err(());
        }

        Ok(())
    }

    fn parse_text_node(&mut self) -> Result<(), ()> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != b'<' {
            self.pos += 1;
        }

        let text = &self.data[start..self.pos];
        if text.is_empty() {
            return Ok(());
        }

        let decoded = decode_entities(text)?;
        if decoded.is_empty() {
            return Ok(());
        }

        let node = self.doc.alloc_text_node(&decoded, xmlElementType::TextNode);
        unsafe {
            self.doc.attach_child(self.stack.last().copied(), node);
        }
        Ok(())
    }

    fn parse_comment(&mut self) -> Result<(), ()> {
        self.expect_sequence(b"<!--")?;
        let start = self.pos;
        while self.pos + 2 < self.data.len() && &self.data[self.pos..self.pos + 3] != b"-->" {
            self.pos += 1;
        }
        if self.pos + 2 >= self.data.len() {
            return Err(());
        }
        let comment = &self.data[start..self.pos];
        self.pos += 3;

        let node = self
            .doc
            .alloc_text_node(comment, xmlElementType::CommentNode);
        unsafe {
            self.doc.attach_child(self.stack.last().copied(), node);
        }
        Ok(())
    }

    fn parse_processing_instruction(&mut self) -> Result<(), ()> {
        self.expect_sequence(b"<?")?;
        while self.pos + 1 < self.data.len() && &self.data[self.pos..self.pos + 2] != b"?>" {
            self.pos += 1;
        }
        if self.pos + 1 >= self.data.len() {
            return Err(());
        }
        self.pos += 2;
        Ok(())
    }

    fn parse_attributes(&mut self) -> Result<Vec<AttributeRecord>, ()> {
        let mut attrs = Vec::new();

        loop {
            self.skip_whitespace();
            if self.pos >= self.data.len() {
                return Err(());
            }

            match self.data[self.pos] {
                b'/' | b'>' => break,
                _ => {
                    let name = self.parse_name()?;
                    self.skip_whitespace();
                    self.expect_char(b'=')?;
                    self.skip_whitespace();
                    let quote = self.next_byte()?;
                    if quote != b'"' && quote != b'\'' {
                        return Err(());
                    }
                    self.pos += 1;
                    let start = self.pos;
                    while self.pos < self.data.len() && self.data[self.pos] != quote {
                        self.pos += 1;
                    }
                    if self.pos >= self.data.len() {
                        return Err(());
                    }
                    let value = &self.data[start..self.pos];
                    self.pos += 1;
                    let decoded = decode_entities(value)?;
                    attrs.push((name, decoded));
                }
            }
        }

        Ok(attrs)
    }

    fn attach_attributes(
        &mut self,
        element: *mut xmlNode,
        attrs: Vec<AttributeRecord>,
    ) -> Result<(), ()> {
        for (name, value) in attrs {
            let attr = self.doc.alloc_attribute(&name);
            if !value.is_empty() {
                let child = self.doc.alloc_text_node(&value, xmlElementType::TextNode);
                unsafe {
                    (*child).parent = ptr::null_mut();
                    (*child).next = ptr::null_mut();
                    (*child).prev = ptr::null_mut();
                    (*attr).children = child;
                    (*attr).last = child;
                }
            }
            unsafe {
                self.doc.append_attribute(element, attr);
            }
        }

        Ok(())
    }

    fn parse_name(&mut self) -> Result<Vec<u8>, ()> {
        if self.pos >= self.data.len() {
            return Err(());
        }

        let start = self.pos;
        if !is_name_start(self.data[self.pos]) {
            return Err(());
        }
        self.pos += 1;
        while self.pos < self.data.len() && is_name_char(self.data[self.pos]) {
            self.pos += 1;
        }

        Ok(self.data[start..self.pos].to_vec())
    }

    fn expect_char(&mut self, expected: u8) -> Result<(), ()> {
        if self.pos >= self.data.len() || self.data[self.pos] != expected {
            return Err(());
        }
        self.pos += 1;
        Ok(())
    }

    fn expect_sequence(&mut self, seq: &[u8]) -> Result<(), ()> {
        if !self.data[self.pos..].starts_with(seq) {
            return Err(());
        }
        self.pos += seq.len();
        Ok(())
    }

    fn consume_char(&mut self, ch: u8) -> bool {
        if self.pos < self.data.len() && self.data[self.pos] == ch {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&self) -> Result<u8, ()> {
        self.data.get(self.pos).copied().ok_or(())
    }
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

fn decode_entities(data: &[u8]) -> Result<Vec<u8>, ()> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if data[i] == b'&' {
            let Some(end) = data[i + 1..].iter().position(|&b| b == b';') else {
                return Err(());
            };
            let entity = &data[i + 1..i + 1 + end];
            i += end + 2;

            if entity.is_empty() {
                return Err(());
            }

            if entity[0] == b'#' {
                let codepoint = if entity.len() > 1 && (entity[1] == b'x' || entity[1] == b'X') {
                    u32::from_str_radix(std::str::from_utf8(&entity[2..]).map_err(|_| ())?, 16)
                        .map_err(|_| ())?
                } else {
                    (std::str::from_utf8(&entity[1..]).map_err(|_| ())?)
                        .parse::<u32>()
                        .map_err(|_| ())?
                };
                push_codepoint(&mut out, codepoint)?;
            } else {
                match entity {
                    b"lt" => out.push(b'<'),
                    b"gt" => out.push(b'>'),
                    b"amp" => out.push(b'&'),
                    b"apos" => out.push(b'\''),
                    b"quot" => out.push(b'"'),
                    _ => {
                        out.push(b'&');
                        out.extend_from_slice(entity);
                        out.push(b';');
                    }
                }
            }
        } else {
            out.push(data[i]);
            i += 1;
        }
    }

    Ok(out)
}

fn push_codepoint(out: &mut Vec<u8>, codepoint: u32) -> Result<(), ()> {
    if let Some(ch) = char::from_u32(codepoint) {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        out.extend_from_slice(encoded.as_bytes());
        Ok(())
    } else {
        Err(())
    }
}

fn is_name_start(byte: u8) -> bool {
    matches!(byte,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'_'
            | b':')
}

fn is_name_char(byte: u8) -> bool {
    is_name_start(byte) || matches!(byte, b'0'..=b'9' | b'-' | b'.')
}

fn node_name_bytes(node: *mut xmlNode) -> Vec<u8> {
    if node.is_null() {
        return Vec::new();
    }

    unsafe {
        if (*node).name.is_null() {
            Vec::new()
        } else {
            CStr::from_ptr((*node).name as *const c_char)
                .to_bytes()
                .to_vec()
        }
    }
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
        sax: ptr::null_mut(),
        user_data: ptr::null_mut(),
        disableSAX: 0,
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
    ctxt.sax = ptr::null_mut();
    ctxt.user_data = ptr::null_mut();
    ctxt.disableSAX = 0;
}

fn register_push_state(ctxt: *mut xmlParserCtxt, state: PushParserState) {
    let mut map = PUSH_PARSER_STATES
        .lock()
        .expect("push parser state mutex poisoned");
    map.insert(ctxt as usize, state);
}

fn clear_push_state(ctxt: *mut xmlParserCtxt) {
    if ctxt.is_null() {
        return;
    }

    if let Ok(mut map) = PUSH_PARSER_STATES.lock() {
        map.remove(&(ctxt as usize));
    }
}
