# Bootstrap Language Prototype

This repository contains the first working compiler for a Rust-inspired language that targets WebAssembly text format (WAT). The MVP focuses on a small, expressive core that we can grow towards both CPU (Wasm) and GPU (WGSL) backends.

## Building

```
cargo build
```

Compilation currently produces the `bootstrapc` binary (via `cargo run`/`cargo build`).

## Usage

```
cargo run -- <source.bp> [-o <output.wat>]
```

If `-o` is omitted the generated WAT module is printed to stdout. The compiler reports lexical, syntactic, and basic typing errors with byte spans.

### Example

Source (`examples/hello.bp`):

```
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    let x: i32 = 10;
    let y: i32 = 32;
    add(x, y)
}
```

Command:

```
cargo run -- examples/hello.bp -o hello.wat
```

The resulting `hello.wat` defines a Wasm module with exported `add` and `main` functions.

## Supported Language Features

- `fn` definitions with typed parameters and return values (`i32`, `i64`, `f32`, `f64`, `bool`, `()`)
- Expression-oriented blocks with lexical scopes, `let`/`mut`, assignments, and trailing expressions
- Expressions: literals, variables, arithmetic/comparison/logical operators, function calls, unary `-` and `!`
- `if` / `else` expressions with boolean conditions
- Early `return` statements

The type checker enforces exact type matches, short-circuit boolean semantics, and requires non-unit functions to end in an expression that supplies the return value.

## Limitations & Next Steps

- No loops, pattern matching, structs/enums, or borrow checking yet
- No WGSL backend; the IR is intentionally shaped so we can add it later
- Only integer remainders are implemented; float `%` emits an error
- Error messages surface byte spans but lack line/column rendering
- Code generation currently exports every function for convenience; we may want finer control later

Planned improvements include lowering user-defined types into linear memory, adding control flow constructs, emitting WGSL from the shared IR, and expanding diagnostics.

For a deeper rationale and layout refer to [`docs/design.md`](docs/design.md).
