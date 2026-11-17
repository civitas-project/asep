; TypeScript/JavaScript tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (top-level only)
(function_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @signature
  return_type: (type_annotation)? @return_type
  body: (statement_block))

(generator_function_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @signature
  return_type: (type_annotation)? @return_type
  body: (statement_block))

; Methods are captured separately
(method_definition
  name: (property_identifier) @name
  parameters: (formal_parameters) @signature
  return_type: (type_annotation)? @return_type
  body: (statement_block))

; Arrow functions (top-level only)
(arrow_function
  parameters: (formal_parameters) @signature
  return_type: (type_annotation)? @return_type
  body: (_))

; Classes
(class_declaration
  name: (type_identifier) @name
  type_parameters: (type_parameters)? @signature
  (class_heritage
    (extends_clause) @signature)?
  body: (class_body))

(class
  name: (type_identifier)? @name
  type_parameters: (type_parameters)? @signature
  body: (class_body))

; Interfaces
(interface_declaration
  name: (type_identifier) @name
  type_parameters: (type_parameters)? @signature
  (extends_type_clause)? @signature
  body: (interface_body))

; Type aliases
(type_alias_declaration
  name: (type_identifier) @name
  value: (type) @signature)

; Enums
(enum_declaration
  name: (identifier) @name
  body: (enum_body))

; Variables (top-level only)
(variable_declaration
  (variable_declarator
    name: (identifier) @name
    value: (_)? @signature))

; Imports
(import_statement
  source: (string) @name)

; Exports (re-exports only)
(export_statement
  source: (string) @name)

; Documentation comments (JSDOC)
(comment) @doc
(#match? @doc "^/\*\*")

; Visibility modifiers
(accessibility_modifier) @visibility
(override_modifier) @visibility

; Attributes/Decorators
(decorator) @attrs