; Java tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Methods
(method_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @signature
  type: (type_identifier)? @return_type
  (#set! kind "fn"))

; Constructors
(constructor_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @signature
  (#set! kind "fn"))

; Classes
(class_declaration
  name: (identifier) @name
  (#set! kind "class"))

; Interfaces
(interface_declaration
  name: (identifier) @name
  (#set! kind "interface"))

; Enums
(enum_declaration
  name: (identifier) @name
  (#set! kind "enum"))

; Records (Java 16+)
(record_declaration
  name: (identifier) @name
  (#set! kind "struct"))

; Constants (static final fields) - match field_declaration with static final modifiers
(field_declaration
  (modifiers) @mods
  (variable_declarator
    name: (identifier) @name)
  value: (_)? @signature
  (#match? @mods "static.*final")
  (#set! kind "const"))

; Fields
(field_declaration
  (variable_declarator
    name: (identifier) @name)
  value: (_)? @signature
  (#set! kind "variable"))

; Imports
(import_declaration
  (scoped_identifier) @name
  (#set! kind "import"))

(import_declaration
  (identifier) @name
  (#set! kind "import"))

; Packages
(package_declaration
  (scoped_identifier) @name
  (#set! kind "module"))

(package_declaration
  (identifier) @name
  (#set! kind "module"))

; Documentation comments (line and block)
(line_comment) @doc
(block_comment) @doc

; Annotations
(annotation) @attrs