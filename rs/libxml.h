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

typedef struct xmlParserCtxt {
  struct xmlDoc *doc;
  int wellFormed;
  int options;
  const char *input;
  int input_size;
  const char *base_url;
  const char *encoding;
} xmlParserCtxt;

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
 * A placeholder implementation of xmlReadMemory.
 *
 * This function is one of the main entry points for parsing an XML document
 * from a buffer in memory. The Rust port currently performs minimal
 * validation, creating a document shell that records the caller supplied
 * metadata.
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
 * Parse a document from a filesystem path, loading the file into memory
 * before delegating to `xmlReadMemory`.
 *
 * # Safety
 * `filename` must be a valid null-terminated string representing a
 * filesystem path that remains live for the duration of this call.
 */
struct xmlDoc *xmlReadFile(const char *filename, const char *encoding, int options);

/**
 * Parse an XML document from an existing file descriptor.
 *
 * # Safety
 * The file descriptor must remain open for the duration of this call. It will
 * **not** be closed by this function.
 */
struct xmlDoc *xmlReadFd(int fd, const char *url, const char *encoding, int options);

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
 * Parse a document using the supplied parser context, synthesising a shell
 * document for downstream consumers.
 *
 * # Safety
 * `ctxt` must be a valid pointer obtained from `xmlCreateMemoryParserCtxt`.
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
