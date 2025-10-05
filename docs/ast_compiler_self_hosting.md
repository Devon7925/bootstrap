# AST compiler self-hosting gaps

The current AST-based compiler only understands a subset of the language that
`compiler/ast_compiler.bp` uses. The following missing features block it from
being able to compile its own source:

- **Boolean types and signatures.** The source defines helpers that return
  `bool` and declares `bool` locals, but the parser hard-codes `i32` as the only
  legal type name, so these declarations cannot be represented yet.【F:compiler/ast_compiler.bp†L23-L117】【F:compiler/ast_compiler.bp†L1055-L1072】
- **Boolean literals.** Functions rely on `true` and `false` values to track
  control-flow state during parsing; the tokenizer and expression lowering still
  need to recognize and emit boolean constants.【F:compiler/ast_compiler.bp†L461-L510】
- **Comparison operators.** Equality/inequality and relational checks (for
  example `==`, `!=`, `<`, `<=`, and `>=`) drive almost every loop and guard in
  the source file, but the AST only models arithmetic expressions, leaving no
  way to encode these comparisons.【F:compiler/ast_compiler.bp†L23-L117】【F:compiler/ast_compiler.bp†L1339-L1413】
- **Logical operators.** The implementation combines conditions with `&&`,
  `||`, and unary `!`, which likewise have no expression variants in the AST
  yet.【F:compiler/ast_compiler.bp†L23-L118】【F:compiler/ast_compiler.bp†L1339-L1413】
- **Bitwise operators and shifts.** Encoding LEB128 values depends on `&`, `|`,
  and `>>`, operations that the expression allocator does not currently
  support.【F:compiler/ast_compiler.bp†L23-L68】【F:compiler/ast_compiler.bp†L1339-L1413】
- **`continue` statements.** Loop bodies (such as comment skipping) use
  `continue;`, a control-flow form that the parser does not yet handle.【F:compiler/ast_compiler.bp†L89-L119】【F:compiler/ast_compiler.bp†L440-L803】
- **`return` statements.** Nearly every helper exits early with `return ...;`,
  so emitting function bodies requires first-class support for explicit returns
  instead of only relying on final block expressions.【F:compiler/ast_compiler.bp†L124-L132】【F:compiler/ast_compiler.bp†L440-L803】
