# Stage1 Failures Missing Diagnostics

This list tracks Stage1 failure paths that a user can trigger via a code-level
regression test and that currently reach the host without populating
`error.failure.detail`. Keeping the list focused on testable scenarios lets us
add coverage alongside future fixes.

## Inline `inline_wasm` argument validation

When `inline_wasm` receives anything other than a literal array of `u8`
constants, the parser aborts by returning `-1` from
`inline_wasm_collect_bytes`/`inline_wasm_literal_byte` without first writing a
failure detail. That covers both non-literal inputs (for example passing a
local variable) and literals that contain values outside the byte range. The
host observes these cases as `Stage1CompileFailure` instances whose
`failure.detail` field is empty, which is why the inline-wasm tests only assert
on the thrown error message today.

- Validation helpers: `compiler/ast_compiler_base.bp`
  (`inline_wasm_literal_byte`, `inline_wasm_collect_bytes`).
- Reproducible coverage: `test/inline_wasm.test.ts` "inline_wasm requires
  literal u8 array" and "inline_wasm enforces u8 range".
