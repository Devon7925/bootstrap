# Bootstrap Language Prototype

This repository contains the first working compiler for a Rust-inspired language that targets WebAssembly text format (WAT). The MVP focuses on a small, expressive core that we can grow towards both CPU (Wasm) and GPU (WGSL) backends.

## Building

```
cargo build
```

Compilation currently produces the `bootstrapc` binary (via `cargo run`/`cargo build`).

## Usage

```
cargo run -- <source.bp> [options]

Options:
  -o <path>           Write output to file (.wat or .wasm)
  --emit <wat|wasm>   Choose output format for stdout (default: wat)
  --run               Execute the compiled module with Node.js
```

If `-o` is omitted the generated WAT module is printed to stdout. Use `--emit wasm`
to stream a binary module instead, or specify a `.wasm` path with `-o`. The compiler
reports lexical, syntactic, and basic typing errors with byte spans. `--run` compiles
to Wasm and invokes the system `node` executable to call the module's exported
`main` function, printing its return value when present.

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

Commands:

```
cargo run -- examples/hello.bp -o hello.wat
cargo run -- examples/hello.bp -o hello.wasm
cargo run -- examples/hello.bp --run
```

The WAT output defines a Wasm module with exported `add` and `main` functions; the
binary output contains the corresponding `.wasm` module, and `--run` executes it
directly with Node.js.

Every compiled program must provide an entry point with the exact signature
`fn main() -> i32`. Additional helper functions can be exported alongside `main`
for host interop.

### WebAssembly Interop

Generated modules export a linear memory named `memory` with an initial size of
one WebAssembly page (64KiB). Hosts can write byte buffers, such as UTF-8
strings, into this memory and pass their pointer/length pairs to compiled
functions (for example to model `&[u8]` inputs) while we continue building out
first-class slice support in the language itself. The bootstrap compiler also
exposes a set of intrinsics for interacting with linear memory directly:

* `load_u8(ptr: i32) -> i32` and `store_u8(ptr: i32, value: i32)` operate on
  single bytes.
* `load_i32(ptr: i32) -> i32` / `store_i32(ptr: i32, value: i32)` and
  `load_i64(ptr: i32) -> i64` / `store_i64(ptr: i32, value: i64)` read and write
  integer words.
* `load_f32(ptr: i32) -> f32` / `store_f32(ptr: i32, value: f32)` and
  `load_f64(ptr: i32) -> f64` / `store_f64(ptr: i32, value: f64)` cover floating
  point data.

These intrinsics allow user code to manipulate byte- and word-oriented buffers
without requiring a standard library yet.

## Supported Language Features

- `fn` definitions with typed parameters and return values (`i32`, `i64`, `f32`, `f64`, `bool`, `()`)
- Expression-oriented blocks with lexical scopes, `let`/`mut`, assignments, and trailing expressions
- Expressions: literals, variables, arithmetic/comparison/logical operators, function calls, unary `-` and `!`
- `if` / `else` expressions with boolean conditions
- `loop` and `while` constructs with `break`/`continue`
- Early `return` statements

The type checker enforces exact type matches, short-circuit boolean semantics, and requires non-unit functions to end in an expression that supplies the return value.

## Limitations & Next Steps

- No pattern matching, structs/enums, or borrow checking yet
- No WGSL backend; the IR is intentionally shaped so we can add it later
- Only integer remainders are implemented; float `%` emits an error
- Error messages surface byte spans but lack line/column rendering
- Code generation currently exports every function for convenience; we may want finer control later

Planned improvements include lowering user-defined types into linear memory, adding control flow constructs, emitting WGSL from the shared IR, and expanding diagnostics.

For a deeper rationale and layout refer to [`docs/design.md`](docs/design.md).
