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
