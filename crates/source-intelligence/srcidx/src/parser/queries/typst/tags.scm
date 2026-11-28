; Typst tagging query for srcidx
; Captures: name, signature, body, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (let with call pattern)
(let
  pattern: (call
    (ident) @name
    (group)? @signature)
  value: (_)? @body
  (#set! kind "fn"))

; Type aliases (let with ident pattern)
(let
  pattern: (ident) @name
  value: (_)? @signature
  (#set! kind "type"))

; Constants (let with ident pattern, but treated as const)
; Note: Typst doesn't distinguish constants from variables in the grammar
; We'll treat all let bindings with ident pattern as variables
; Constants are handled by the test expecting 0 consts

; Variables (let with ident pattern)
(let
  pattern: (ident) @name
  value: (_)? @signature
  (#set! kind "variable"))

; Imports
(import
  import: (string) @name
  (#set! kind "import"))

; Documentation comments (JSDOC)
(comment) @doc