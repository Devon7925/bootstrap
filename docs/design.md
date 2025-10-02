# Language Bootstrap Design

This document captures the initial plan for the bootstrap compiler. The overall goal is to provide a minimal-yet-compilable subset of the language described in `concept.md` so that we can iterate towards richer features quickly.

## Guiding Goals

- **Primary targets**: WebAssembly (text format `.wat`) for CPU execution in the browser or server runtimes. The same typed AST will eventually lower to WGSL for GPU kernels. WGSL support is deferred but we keep its constraints in mind.
- **Rust-inspired ergonomics**: surface syntax borrows from Rust (blocks, `fn`, `let`, trailing expression returns, `mut` keyword) while avoiding complex borrow checker rules in this bootstrap stage.
- **Typed core**: explicit types everywhere. Generics are replaced by `const` evaluated shapes later, but for now we hand-pick a small type palette that matches Wasm MVP types.

## MVP Feature Set

- **Program**: a list of `fn` definitions. The entry point is `fn main() -> i32`.
- **Types**: `i32`, `i64`, `f32`, `f64`, user-defined `struct` and `enum` declarations are parsed but not yet lowered. Borrowing & pointers are parsed as syntax stubs only.
- **Statements**: `let` binding (immutable or `mut`), assignment, block, `return`, `break`, `continue`, expression statements.
- **Expressions**: integer/float literals, variable ref, unary/binary arithmetic (`+ - * / %`), comparison (`== != < <= > >=`), logical (`&& || !`), call expression.
- **Control Flow**: `if { } else { }` expressions, `loop { }`, `break`, `continue`, `while ( condition ) { ... }` desugared into loops.
- **Intrinsics**: `load_u8`/`store_u8`, `load_i32`/`store_i32`, `load_i64`/`store_i64`, `load_f32`/`store_f32`, and
  `load_f64`/`store_f64` provide direct access to linear memory while we build richer data structures.

## Compiler Pipeline

```
source -> Lexer -> Parser -> AST -> Type Checker -> HIR -> Codegen (WAT)
```

- **Lexer**: produces tokens with span information.
- **Parser**: Pratt-style expression parser and recursive-descent for higher-level constructs.
- **AST**: high-level tree capturing syntax.
- **Type Checker**: resolves names in a scoped environment, infers expression types (within explicit annotation limits) and validates function signatures.
- **HIR**: for the bootstrap we reuse the AST with resolved types (no separate structure yet). A later pass can lower into a control-flow graph for optimisations.
- **Codegen**: converts the typed AST into `.wat` text. Currently limited to arithmetic expressions and block control flow that Wasm MVP supports easily.

## Code Organization

```
src/
  main.rs      // CLI entrypoint
  lib.rs       // Re-exports compiler API
  lexer.rs
  parser/
    mod.rs
    expr.rs
    stmt.rs
  ast.rs
  typeck.rs
  hir.rs       // placeholder for future lowering
  codegen/
    mod.rs
    wat.rs
  error.rs
```

## Command-Line UX

```
bootstrapc <input.bp> -o <output.wat>
```

- Parses the input file, runs semantic analysis, and writes the generated WAT. Diagnostic output goes to stderr.

## Future Work

- Add `struct`/`enum` lowering to Wasm linear memory.
- Introduce borrow checking and lifetime annotations.
- Extend codegen with function imports/exports and host interop.
- Emit WGSL alongside Wasm once we settle on a shared IR.
- Add const-evaluated types for generic-like capabilities.

