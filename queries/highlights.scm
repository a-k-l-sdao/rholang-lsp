; highlights.scm — Tree-sitter highlight queries for Rholang

; Keywords
"new" @keyword
"in" @keyword
"contract" @keyword
"for" @keyword
"select" @keyword
"match" @keyword
"if" @keyword
"else" @keyword
"let" @keyword

(bundle_write) @keyword
(bundle_read) @keyword
(bundle_equiv) @keyword
(bundle_read_write) @keyword

"not" @keyword.operator
"and" @keyword.operator
"or" @keyword.operator
"matches" @keyword.operator

; Literals
(string_literal) @string
(uri_literal) @string.special
(long_literal) @number
(bool_literal) @boolean
(nil) @constant.builtin

; Types
(simple_type) @type.builtin
"Set" @type.builtin

; Variables and names
(wildcard) @variable.builtin
(name_decl (var) @variable.parameter)
(contract name: (var) @function)
(method name: (var) @function.method)
(var_ref kind: (var_ref_kind) @punctuation.special var: (var) @variable)
(var) @variable

; Comments
(line_comment) @comment
(block_comment) @comment
