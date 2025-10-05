# AST bootstrap blockers

Self-hosting the AST compiler still fails in
`tests/bootstrap_ast.rs::ast_compiler_compiler_bootstraps`. Two classes of
issues currently prevent the stage-one compiler from successfully compiling its
own source.

## Capacity limits must be raised

The `tests/bootstrap_ast.rs::ast_compiler_compiler_bootstraps` test currently
fails when the compiler attempts to compile `compiler/ast_compiler.bp` with the
self-hosted AST compiler. The stage-one compiler embedded in
`compiler/ast_compiler.bp` imposes very small fixed capacities on several of its
in-memory data structures:

* `ast_max_functions()` returns 16, limiting the compiler to just 16 function
  definitions.【F:compiler/ast_compiler.bp†L1696-L1702】
* `ast_names_capacity()` and `ast_call_data_capacity()` are each 512 bytes,
  which is far too small to store the identifiers and call metadata for the 167
  functions in the AST compiler.【F:compiler/ast_compiler.bp†L1704-L1712】
* `ast_expr_capacity()` allows only 256 expression nodes for the entire program,
  which is quickly exceeded by the AST compiler's control flow and expression
  complexity.【F:compiler/ast_compiler.bp†L1824-L1832】
* `max_params()` caps function parameter lists at 16 entries, but
  `parse_block_expression_body`'s signature alone needs 18 parameters, so the
  parser rejects the definition long before it can reach the body.【F:compiler/ast_compiler.bp†L381-L383】【F:compiler/ast_compiler.bp†L791-L809】
* `max_locals()` limits each scope to 64 local bindings, while
  `parse_block_expression_body` declares well over one hundred temporaries as it
  walks block statements. Even if the parameter limit were lifted, the parser
  would still abort once it crosses this much lower ceiling.【F:compiler/ast_compiler.bp†L710-L824】【F:compiler/ast_compiler.bp†L890-L1008】

These increases remove the most obvious capacity cliffs but do not yet make the
bootstrap succeed on their own.

## Functions without explicit return types cannot be parsed

`parse_function` still assumes every function declares a return type with `->`
and that the body supplies a trailing expression. When the parser reaches
helpers such as `ast_reset`, `ast_write_function_entry`, or `initialize_layout`
— all of which omit an explicit return type and end their bodies with
statements — it returns `-1` and aborts the compilation.【F:compiler/ast_compiler.bp†L3706-L3850】【F:compiler/ast_compiler.bp†L1719-L1835】

Supporting “void” functions will require loosening the grammar so the return
type is optional and allowing blocks to omit a final value expression. Until
that behaviour is implemented, the bootstrap test will continue to observe
`CompileFailure { produced_len: -1, functions: 0, ... }` despite the higher
memory limits.

## Statement `if` handling prevents parsing the compiler source

Even if the AST arena is enlarged enough to keep the parser from running out of
space, the self-hosted parser currently requires every block to end in a value
expression. When `parse_block_expression_body` sees a closing brace without
`have_value_expr` set, it immediately reports an error unless the block was
explicitly allowed to be value-less (e.g. loop bodies) or the previous
statement provably diverges.【F:compiler/ast_compiler.bp†L826-L875】 The AST
compiler source, however, uses `if … { … };` as a statement in many places.
One of the earliest occurrences appears in `skip_whitespace`, which contains a
chain of nested `if` statements that end with semicolons because they are
intended to mutate local state, not to produce a value.【F:compiler/ast_compiler.bp†L96-L112】

When the self-hosted parser reaches the closing brace of these statement
branches it does not have a value expression to record, the diverges check does
not apply, and `allow_empty_final_expr` is `0`, so the parser aborts the
compilation. This happens before any functions are emitted, which matches the
`CompileFailure { produced_len: -1, functions: 0, … }` observed during the
bootstrap test. Fixing this requires teaching the parser to distinguish between
`if` expressions and `if` statements (or otherwise permitting non-value blocks)
so that it can successfully parse the compiler's own source code.
