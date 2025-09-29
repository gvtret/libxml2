#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum xmlAttributeType {
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
} xmlAttributeType;

typedef enum xmlElementType {
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
} xmlElementType;

typedef struct xmlNs {
  struct xmlNs *next;
  enum xmlElementType type_;
  const uint8_t *href;
  const uint8_t *prefix;
  void *_private;
  struct xmlDoc *context;
} xmlNs;

typedef struct xmlAttr {
  void *_private;
  enum xmlElementType type_;
  const uint8_t *name;
  struct xmlNode *children;
  struct xmlNode *last;
  struct xmlNode *parent;
  struct xmlAttr *next;
  struct xmlAttr *prev;
  struct xmlDoc *doc;
  struct xmlNs *ns;
  enum xmlAttributeType atype;
  void *psvi;
} xmlAttr;

typedef struct xmlNode {
  void *_private;
  enum xmlElementType type_;
  const uint8_t *name;
  struct xmlNode *children;
  struct xmlNode *last;
  struct xmlNode *parent;
  struct xmlNode *next;
  struct xmlNode *prev;
  struct xmlDoc *doc;
  struct xmlNs *ns;
  uint8_t *content;
  struct xmlAttr *properties;
  struct xmlNs *nsDef;
  void *psvi;
  unsigned short line;
  unsigned short extra;
} xmlNode;

typedef struct xmlDoc {
  void *_private;
  enum xmlElementType type_;
  char *name;
  struct xmlNode *children;
  struct xmlNode *last;
  struct xmlNode *parent;
  struct xmlNode *next;
  struct xmlNode *prev;
  struct xmlDoc *doc;
  int compression;
  int standalone;
  void *intSubset;
  void *extSubset;
  struct xmlNs *oldNs;
  const uint8_t *version;
  const uint8_t *encoding;
  void *ids;
  void *refs;
  const uint8_t *URL;
  int charset;
  void *dict;
  void *psvi;
  int parseFlags;
  int properties;
} xmlDoc;

typedef struct xmlSAXHandler {
  void *_private;
} xmlSAXHandler;

typedef struct xmlParserCtxt {
  struct xmlDoc *doc;
  int wellFormed;
  int options;
  const char *input;
  int input_size;
  const char *base_url;
  const char *encoding;
  struct xmlSAXHandler *sax;
  void *user_data;
  int disableSAX;
} xmlParserCtxt;

typedef int (*xmlInputReadCallback)(void *context, char *buffer, int len);

typedef int (*xmlInputCloseCallback)(void *context);

/**
 * Allocate a new document populated with the provided XML version.
 *
 * # Safety
 * `version` must be either null or a pointer to a valid null-terminated
 * string.
 */
struct xmlDoc *xmlNewDoc(const uint8_t *version);

/**
 * Frees the memory allocated for an xmlDoc.
 *
 * # Safety
 * The caller must ensure that `doc` either originated from one of the Rust
 * constructors and that it is not freed multiple times.
 */
void xmlFreeDoc(struct xmlDoc *doc);

/**
 * Parse an XML document stored entirely in memory and return a fully
 * populated `xmlDoc` tree.
 *
 * # Safety
 * The caller must supply valid pointers for the input buffer and optional
 * strings (which may be null) following libxml2's C API contracts. The
 * returned pointer must be released with `xmlFreeDoc`.
 */
struct xmlDoc *xmlReadMemory(const char *buffer,
                             int size,
                             const char *url,
                             const char *encoding,
                             int options);

/**
 * Create a push-style parser context capable of consuming data incrementally.
 *
 * # Safety
 * `chunk` must either be null (when `size` is zero) or reference a readable
 * memory region with at least `size` bytes. The returned context must be
 * released with `xmlFreeParserCtxt`.
 */
struct xmlParserCtxt *xmlCreatePushParserCtxt(struct xmlSAXHandler *sax,
                                              void *user_data,
                                              const char *chunk,
                                              int size,
                                              const char *filename);

/**
 * Feed data into an existing push-style parser context.
 *
 * # Safety
 * `chunk` must be either null (when `size` is zero) or point to at least
 * `size` readable bytes. Set `terminate` to a non-zero value once no more data
 * will be supplied.
 */
int xmlParseChunk(struct xmlParserCtxt *ctxt, const char *chunk, int size, int terminate);

/**
 * Halt any further parsing activity on the supplied parser context.
 *
 * # Safety
 * `ctxt` must be either null or a valid parser context pointer obtained from
 * one of the Rust constructors.
 */
void xmlStopParser(struct xmlParserCtxt *ctxt);

/**
 * Resume parsing on a push-style parser context that was previously stopped.
 *
 * # Safety
 * `ctxt` must be either null or a valid pointer obtained from one of the Rust
 * constructors. Returns `0` on success and `-1` if the parser cannot be
 * resumed (for example, if it has already been terminated).
 */
int xmlResumeParser(struct xmlParserCtxt *ctxt);

/**
 * Parse a buffer in recovery mode, mirroring `xmlRecoverMemory`.
 *
 * # Safety
 * Delegates to `xmlReadMemory`; see that function for requirements.
 */
struct xmlDoc *xmlRecoverMemory(const char *buffer,
                                int size,
                                const char *url,
                                const char *encoding,
                                int options);

/**
 * Initialise the global parser state bookkeeping.
 *
 * # Safety
 * Matches the C ABI contract: may be called from any thread without prior
 * initialisation. The function performs no memory unsafe operations.
 */
void xmlInitParser(void);

/**
 * Tear down the global parser bookkeeping established by `xmlInitParser`.
 *
 * # Safety
 * Safe to call multiple times and from any thread, mirroring the semantics of
 * the legacy C implementation.
 */
void xmlCleanupParser(void);

/**
 * Parse a full XML document provided as a null-terminated UTF-8 buffer.
 *
 * # Safety
 * `cur` must point to a valid, null-terminated string containing the
 * serialized document. The returned pointer must be freed with
 * `xmlFreeDoc`.
 */
struct xmlDoc *xmlReadDoc(const uint8_t *cur, const char *url, const char *encoding, int options);

/**
 * Parse a null-terminated buffer in recovery mode.
 *
 * # Safety
 * Delegates to `xmlReadDoc`; see that function for requirements.
 */
struct xmlDoc *xmlRecoverDoc(const uint8_t *cur,
                             const char *url,
                             const char *encoding,
                             int options);

/**
 * Parse a document from a filesystem path, loading the file into memory
 * before delegating to `xmlReadMemory`.
 *
 * # Safety
 * `filename` must be a valid null-terminated string representing a
 * filesystem path that remains live for the duration of this call.
 */
struct xmlDoc *xmlReadFile(const char *filename, const char *encoding, int options);

/**
 * Parse a document from disk using a SAX handler.
 *
 * # Safety
 * `sax` and `user_data` may be null and are currently unused by the Rust
 * placeholder implementation. `filename` must be a valid null-terminated
 * string. Returns `0` on success and `-1` on failure, mirroring libxml2's C
 * API contract.
 */
int xmlSAXUserParseFile(void *sax, void *user_data, const char *filename);

/**
 * Parse a document from a filesystem path in recovery mode.
 *
 * # Safety
 * Delegates to `xmlReadFile`; see that function for requirements.
 */
struct xmlDoc *xmlRecoverFile(const char *filename, const char *encoding, int options);

/**
 * Parse an XML document from an existing file descriptor.
 *
 * # Safety
 * The file descriptor must remain open for the duration of this call. It will
 * **not** be closed by this function.
 */
struct xmlDoc *xmlReadFd(int fd, const char *url, const char *encoding, int options);

/**
 * Parse an in-memory document using a SAX handler.
 *
 * # Safety
 * The placeholder parser validates the buffer using `xmlReadMemory` and does
 * not trigger callbacks on the provided SAX handler. `buffer` must either be
 * null (when `size` is zero) or reference a readable memory region of `size`
 * bytes. Returns `0` on success and `-1` otherwise.
 */
int xmlSAXUserParseMemory(void *sax, void *user_data, const char *buffer, int size);

/**
 * Parse a document from custom I/O callbacks, mirroring `xmlReadIO`.
 *
 * # Safety
 * `ioread` must be a valid callback that reads from `ioctx` into the provided
 * buffer. `ioclose`, when non-null, is invoked after reading completes (even
 * on error). The returned document must be released with `xmlFreeDoc`.
 */
struct xmlDoc *xmlReadIO(xmlInputReadCallback ioread,
                         xmlInputCloseCallback ioclose,
                         void *ioctx,
                         const char *url,
                         const char *encoding,
                         int options);

/**
 * Parse a document held entirely in memory, mirroring libxml2's legacy API.
 *
 * # Safety
 * Delegates to `xmlReadMemory`; see that function for requirements.
 */
struct xmlDoc *xmlParseMemory(const char *buffer, int size);

/**
 * Parse a document from a null-terminated buffer, returning a constructed
 * `xmlDoc` on success.
 *
 * # Safety
 * `cur` must reference a valid, null-terminated string. The caller owns the
 * returned document and must release it with `xmlFreeDoc`.
 */
struct xmlDoc *xmlParseDoc(const uint8_t *cur);

/**
 * Parse a document directly from a file path using the default parsing
 * options.
 *
 * # Safety
 * `filename` must be a valid null-terminated string representing a
 * filesystem path that remains live for the duration of this call.
 */
struct xmlDoc *xmlParseFile(const char *filename);

/**
 * Parse XML content into the provided parser context from an in-memory buffer.
 *
 * # Safety
 * `ctxt` must be a valid pointer obtained from `xmlCreateMemoryParserCtxt` (or an
 * equivalent constructor once streaming support lands).
 */
struct xmlDoc *xmlCtxtReadMemory(struct xmlParserCtxt *ctxt,
                                 const char *buffer,
                                 int size,
                                 const char *url,
                                 const char *encoding,
                                 int options);

/**
 * Parse a null-terminated document string using an existing parser context.
 *
 * # Safety
 * `cur` must point to a valid, null-terminated buffer.
 */
struct xmlDoc *xmlCtxtReadDoc(struct xmlParserCtxt *ctxt,
                              const uint8_t *cur,
                              const char *url,
                              const char *encoding,
                              int options);

/**
 * Load and parse a document from a file path using the supplied parser context.
 *
 * # Safety
 * `filename` must be a valid null-terminated string.
 */
struct xmlDoc *xmlCtxtReadFile(struct xmlParserCtxt *ctxt,
                               const char *filename,
                               const char *encoding,
                               int options);

/**
 * Parse an XML document using the supplied context and file descriptor.
 *
 * # Safety
 * The file descriptor must stay valid for the duration of the call and is not
 * closed when parsing completes.
 */
struct xmlDoc *xmlCtxtReadFd(struct xmlParserCtxt *ctxt,
                             int fd,
                             const char *url,
                             const char *encoding,
                             int options);

/**
 * Parse an XML document into an existing context using custom I/O callbacks.
 *
 * # Safety
 * `ctxt` must be a valid parser context and `ioread` must read from `ioctx`
 * according to libxml2's callback contracts. `ioclose`, when provided, is
 * invoked after reading completes (even on error).
 */
struct xmlDoc *xmlCtxtReadIO(struct xmlParserCtxt *ctxt,
                             xmlInputReadCallback ioread,
                             xmlInputCloseCallback ioclose,
                             void *ioctx,
                             const char *url,
                             const char *encoding,
                             int options);

/**
 * Allocate a fresh parser context initialised with default state.
 *
 * # Safety
 * Returns a raw pointer that must be released with `xmlFreeParserCtxt`. The
 * caller is responsible for ensuring the context is not leaked or freed twice.
 */
struct xmlParserCtxt *xmlNewParserCtxt(void);

/**
 * Reset an existing parser context to its initial state.
 *
 * # Safety
 * `ctxt` must be either null or a pointer obtained from one of the parser
 * context constructors. Passing any other pointer is undefined behaviour.
 */
int xmlInitParserCtxt(struct xmlParserCtxt *ctxt);

/**
 * Clear the transient parse state stored in a parser context.
 *
 * # Safety
 * `ctxt` must be either null or a valid parser context pointer previously
 * returned by the Rust constructors.
 */
void xmlClearParserCtxt(struct xmlParserCtxt *ctxt);

/**
 * Create a parser context for parsing from an in-memory buffer.
 *
 * # Safety
 * `buffer` must either be null (when `size` is zero) or point to at least
 * `size` bytes of readable memory. The returned context must eventually be
 * released with `xmlFreeParserCtxt`.
 */
struct xmlParserCtxt *xmlCreateMemoryParserCtxt(const char *buffer, int size);

/**
 * Parse the buffer registered on the supplied context and produce a document
 * tree when the input is well-formed.
 *
 * # Safety
 * `ctxt` must be a valid pointer obtained from the parser-context
 * constructors. The context's `input` and `input_size` fields must describe a
 * readable memory region that remains accessible for the duration of this
 * call.
 */
int xmlParseDocument(struct xmlParserCtxt *ctxt);

/**
 * Release the resources held by a parser context.
 *
 * # Safety
 * `ctxt` must be null or a pointer obtained from `xmlCreateMemoryParserCtxt`.
 */
void xmlFreeParserCtxt(struct xmlParserCtxt *ctxt);

/**
 * Create a parser context primed with a null-terminated in-memory document.
 *
 * # Safety
 * `cur` must be a valid pointer to a null-terminated buffer that remains
 * accessible for the lifetime of the parser context unless replaced by other
 * parsing routines. The returned context must be freed with
 * `xmlFreeParserCtxt`.
 */
struct xmlParserCtxt *xmlCreateDocParserCtxt(const uint8_t *cur);
