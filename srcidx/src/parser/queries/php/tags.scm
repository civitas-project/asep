; PHP tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (top-level)
(function_definition
  name: (name) @name
  parameters: (formal_parameters) @signature
  return_type: (type)? @return_type
  (#set! kind "fn"))

; Methods
(method_declaration
  name: (name) @name
  parameters: (formal_parameters) @signature
  return_type: (type)? @return_type
  (#set! kind "fn"))

; Classes
(class_declaration
  name: (name) @name
  (#set! kind "class"))

; Interfaces
(interface_declaration
  name: (name) @name
  (#set! kind "interface"))

; Traits
(trait_declaration
  name: (name) @name
  (#set! kind "trait"))

; Enums (PHP 8.1+)
(enum_declaration
  name: (name) @name
  (#set! kind "enum"))

; Constants
(const_declaration
  (const_element (name) @name)
  value: (_)? @signature
  (#set! kind "const"))

; Properties
(property_declaration
  (property_element (variable_name (name) @name))
  value: (_)? @signature
  (#set! kind "variable"))

; Use statements (imports) - different patterns for different forms
(namespace_use_declaration
  (namespace_use_clause
    (name) @name)
  (#set! kind "import"))

(namespace_use_declaration
  (namespace_use_clause
    (qualified_name) @name)
  (#set! kind "import"))

; Namespaces
(namespace_definition
  name: (namespace_name) @name
  (#set! kind "module"))

; Documentation comments
(comment) @doc

; Attributes (PHP 8.0+)
(attribute) @attrs