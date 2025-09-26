#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum ExtxmlAttributeType {
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
} ExtxmlAttributeType;

typedef enum ExtxmlElementType {
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
} ExtxmlElementType;

typedef struct ExtxmlNs {
  struct ExtxmlNs *next;
  enum ExtxmlElementType type_;
  const uint8_t *href;
  const uint8_t *prefix;
  void *_private;
  struct ExtxmlDoc *context;
} ExtxmlNs;

typedef struct ExtxmlAttr {
  void *_private;
  enum ExtxmlElementType type_;
  const uint8_t *name;
  struct ExtxmlNode *children;
  struct ExtxmlNode *last;
  struct ExtxmlNode *parent;
  struct ExtxmlAttr *next;
  struct ExtxmlAttr *prev;
  struct ExtxmlDoc *doc;
  struct ExtxmlNs *ns;
  enum ExtxmlAttributeType atype;
  void *psvi;
} ExtxmlAttr;

typedef struct ExtxmlNode {
  void *_private;
  enum ExtxmlElementType type_;
  const uint8_t *name;
  struct ExtxmlNode *children;
  struct ExtxmlNode *last;
  struct ExtxmlNode *parent;
  struct ExtxmlNode *next;
  struct ExtxmlNode *prev;
  struct ExtxmlDoc *doc;
  struct ExtxmlNs *ns;
  uint8_t *content;
  struct ExtxmlAttr *properties;
  struct ExtxmlNs *nsDef;
  void *psvi;
  unsigned short line;
  unsigned short extra;
} ExtxmlNode;

typedef struct ExtxmlDoc {
  void *_private;
  enum ExtxmlElementType type_;
  char *name;
  struct ExtxmlNode *children;
  struct ExtxmlNode *last;
  struct ExtxmlNode *parent;
  struct ExtxmlNode *next;
  struct ExtxmlNode *prev;
  struct ExtxmlDoc *doc;
  int compression;
  int standalone;
  void *intSubset;
  void *extSubset;
  struct ExtxmlNs *oldNs;
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
} ExtxmlDoc;

typedef struct ExtxmlParserCtxt {
  struct ExtxmlDoc *doc;
  int wellFormed;
  int options;
} ExtxmlParserCtxt;

/**
 * A placeholder implementation of xmlReadMemory.
 *
 * This function is one of the main entry points for parsing an XML document
 * from a buffer in memory. For now, it creates and returns a dummy document
 * to allow us to test the FFI linkage.
 */
struct ExtxmlDoc *xmlReadMemory(const char *_buffer,
                                int _size,
                                const char *_url,
                                const char *_encoding,
                                int options);

/**
 * Frees the memory allocated for an xmlDoc.
 *
 * This function is essential for preventing memory leaks when the C test code
 * cleans up the documents created by `xmlReadMemory`.
 */
void xmlFreeDoc(struct ExtxmlDoc *doc);
