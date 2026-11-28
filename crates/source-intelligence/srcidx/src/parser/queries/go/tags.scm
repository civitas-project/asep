; Go tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (top-level)
(function_declaration
  name: (identifier) @name
  parameters: (parameter_list) @signature
  result: (_)? @return_type
  (#set! kind "fn"))

; Methods
(method_declaration
  name: (field_identifier) @name
  parameters: (parameter_list) @signature
  result: (_)? @return_type
  (#set! kind "fn"))

; Structs (type_declaration with type_spec containing struct_type)
(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (struct_type) @signature)
  (#set! kind "struct"))

; Interfaces (type_declaration with type_spec containing interface_type)
(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (interface_type) @signature)
  (#set! kind "interface"))

; Type aliases (type_declaration with type_alias child)
(type_declaration
  (type_alias
    name: (type_identifier) @name
    type: (_) @signature)
  (#set! kind "type"))

; Constants
(const_declaration
  (const_spec
    name: (identifier) @name
    value: (_)? @signature)
  (#set! kind "const"))

; Variables
(var_declaration
  (var_spec
    name: (identifier) @name
    value: (_)? @signature)
  (#set! kind "variable"))

; Short variable declarations (:=)
(short_var_declaration
  left: (expression_list) @name
  right: (expression_list)? @signature
  (#set! kind "variable"))

; Imports (single import_spec)
(import_declaration
  (import_spec
    path: (interpreted_string_literal) @name)
  (#set! kind "import"))

; Imports (import_spec_list with multiple import_spec)
(import_declaration
  (import_spec_list
    (import_spec
      path: (interpreted_string_literal) @name))
  (#set! kind "import"))

; Documentation comments
(comment) @doc