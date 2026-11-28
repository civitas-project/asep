; Python tagging query for srcidx
; Captures: name, signature, return_type, doc, visibility, attrs
; Kind is determined by pattern index via pattern_kind_map

; Functions (top-level only)
(function_definition
  name: (identifier) @name
  parameters: (parameters) @signature
  return_type: (type)? @return_type
  body: (block)
  (#not-ancestor? @name "class_definition")
  (#set! kind "fn"))

; Decorated functions (top-level only)
(decorated_definition
  (function_definition
    name: (identifier) @name
    parameters: (parameters) @signature
    return_type: (type)? @return_type
    body: (block))
  (#not-ancestor? @name "class_definition")
  (#set! kind "fn"))

; Class methods
(class_definition
  body: (block
    (function_definition
      name: (identifier) @name
      parameters: (parameters) @signature
      return_type: (type)? @return_type
      body: (block))))

; Classes (with superclasses)
(class_definition
  name: (identifier) @name
  superclasses: (argument_list) @signature
  body: (block)
  (#set! kind "class"))

; Classes (without superclasses)
(class_definition
  name: (identifier) @name
  body: (block)
  (#set! kind "class"))

; Decorators - capture the decorator node
(decorator) @name

; Imports - combined pattern for both dotted_name and aliased_import
(import_statement
  name: [(dotted_name) (aliased_import name: (dotted_name))] @name)

(import_from_statement
  module_name: (dotted_name) @name)

; Variables
(assignment
  left: (identifier) @name
  right: (_)? @signature)

(augmented_assignment
  left: (identifier) @name
  right: (_)? @signature)

; Type aliases (type_alias_statement has 'left' field for the alias name, 'right' for value)
(type_alias_statement
  left: (type
    (identifier) @name)
  right: (type) @signature)

; Documentation comments
(comment) @doc

; Attributes/Decorators
(decorator) @attrs