# Compilation Pipeline Overview

The Bootstrap compiler turns source text into a WebAssembly module through a
series of linear, memory-backed stages. Each stage operates in-place on the
compiler's arenas so that every pass can share data without additional
allocation.

## 1. Layout Initialisation
The compiler reserves space in its linear memory for temporary buffers, the
abstract syntax tree (AST), and output sections. This setup happens in
`compile_impl` before any user code is inspected.

## 2. Parsing Modules
`parse_program` tokenises and parses the input source into an arena-backed AST.
The parser resolves `use` declarations immediately, recursively parsing imported
modules so that the current compile has a complete view of all available
functions and types.

## 3. Constant Interpretation Preparation
After parsing, `interpret_program_constants` iterates through the AST to gather
context for constant evaluation. Today the pass is a scaffold that walks
constants, functions, and expressions without changing behaviour, but it will be
extended to interpret constant values in a future update.

## 4. Semantic Validation
Once parsing completes, `validate_program` walks the AST to resolve expression
types, enforce control-flow invariants, and bind call sites to their targets.
This pass ensures the emitter can assume the AST is type-safe and structurally
sound.

## 5. Type Metadata Extraction
With a validated AST in place, `write_type_metadata` serialises information about
composite types (arrays, tuples, and other heap values). The WebAssembly emitter
uses this metadata when it constructs the module's type and heap sections.

## 6. WebAssembly Emission
Finally, `emit_program` writes the WebAssembly binary. It emits the module
header, type section, function bodies, and any additional data segments directly
into the preallocated output buffer.

At the end of this pipeline the output buffer contains a complete WebAssembly
module that the host can pass to a runtime or further toolchain stages.
