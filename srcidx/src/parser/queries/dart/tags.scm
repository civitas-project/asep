; Dart tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (top-level)
(function_declaration
  signature: (function_signature
    name: (identifier) @name
    parameters: (formal_parameter_list) @signature
    return_type: (type)? @return_type)
  (#set! kind "fn"))

; Methods
(method_declaration
  signature: (method_signature
    (function_signature
      name: (identifier) @name
      parameters: (formal_parameter_list) @signature
      return_type: (type)? @return_type))
  (#set! kind "fn"))

; Constructors
(constructor_signature
  name: (identifier) @name
  parameters: (formal_parameter_list) @signature
  (#set! kind "fn"))

; Classes
(class_declaration
  name: (identifier) @name
  superclass: (superclass)? @signature
  (#set! kind "class"))

; Enums
(enum_declaration
  name: (identifier) @name
  (#set! kind "enum"))

; Mixins
(mixin_declaration
  name: (identifier) @name
  (#set! kind "mixin"))

; Extensions
(extension_declaration
  name: (identifier) @name
  (#set! kind "extension"))

; Type aliases (type_alias has children: type_identifier for name, type for value)
(type_alias
  (type_identifier) @name
  (type)? @signature
  (#set! kind "typedef"))

; Imports (use import_or_export which is named)
(import_or_export
  (library_import
    (import_specification
      uri: (configurable_uri
        (uri
          (string_literal) @name))))
  (#set! kind "import"))

; Documentation comments
(comment) @doc

; Metadata/Annotations (use annotation, not metadata)
(annotation) @attrs