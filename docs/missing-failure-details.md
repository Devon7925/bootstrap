# Gaps in Stage1 Failure Diagnostics

The Stage1 toolchain reports structured failures through `error.failure.detail`, but several code
paths still return a `Stage1CompileFailure` without populating the diagnostic string. The situations
below currently drop detail text and should be tackled individually so fixes remain tractable.

## Module loading APIs

### `compileFromPath`

- **Downstream pipeline status codes.** Any negative status propagated from `compile_impl`
  (parsing, constant evaluation, validation, metadata generation, or code emission) ultimately
  surfaces through the host without extra context when the callee omits a detail message.
  【F:compiler/ast_compiler.bp†L156-L158】
- **Memory reservation failures now emit detail.** The linear-memory reservation branch writes a
  message today, so no additional work is required for this case. 【F:compiler/ast_compiler.bp†L149-L154】

## Stage1 pipeline phases lacking guaranteed diagnostics

`compile_impl` forwards negative statuses from each compilation phase. Several callees still return
`-1` without guaranteeing that a detail string was written first, producing silent failures in the
host wrapper.

- **Constant interpretation errors.** `interpret_program_constants` can return `< 0` and relies on
  the interpreter to explain the failure; several error exits only propagate status codes.
  【F:compiler/ast_compiler.bp†L28-L31】【F:compiler/ast_semantics.bp†L849-L907】
- **Semantic validation checks.** The semantic validator forwards `-1` for many guardrail failures
  without always setting `failure.detail`. 【F:compiler/ast_compiler.bp†L31-L34】【F:compiler/ast_semantics.bp†L925-L1096】
- **Function accounting.** A negative result from `ast_functions_count` leads to an unannotated early
  return. 【F:compiler/ast_compiler.bp†L34-L37】【F:compiler/ast_compiler_base.bp†L3556-L3562】
- **Metadata emission failures.** `write_type_metadata` can produce `< 0` when buffers overflow or
  layout checks fail without recording why. 【F:compiler/ast_compiler.bp†L37-L40】【F:compiler/wasm_output.bp†L3272-L3348】
- **Code generation errors.** A non-positive byte count from `emit_program` is surfaced directly to
  the host with no additional detail when the emitter stops early. 【F:compiler/ast_compiler.bp†L40-L47】【F:compiler/wasm_output.bp†L3301-L3358】

These gaps highlight the remaining guardrails that need bespoke diagnostics so the host can surface
actionable error messages to users.
