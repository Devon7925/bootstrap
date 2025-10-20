# Gaps in Stage1 Failure Diagnostics

The Stage1 toolchain reports structured failures through `error.failure.detail`, but several code
paths still return a `Stage1CompileFailure` without populating the diagnostic string. The situations
below currently drop detail text and should be tackled individually so fixes remain tractable.

## Module loading APIs

### `loadModuleFromSource`

- **Null module path pointer.** A zero or negative pointer causes an immediate `-1` return with no
  attempt to write into the failure buffer. 【F:compiler/ast_compiler.bp†L59-L63】
- **Missing module content.** When the provided content pointer is zero or negative, the helper exits
  without providing context for the failure. 【F:compiler/ast_compiler.bp†L59-L63】
- **Negative content length.** If `string_length(content_ptr)` reports a negative length the function
  returns `-1` without populating `failure.detail`. 【F:compiler/ast_compiler.bp†L72-L75】
- **Module table capacity reached.** Hitting `MODULE_MAX_COUNT` silently rejects the registration
  request. 【F:compiler/ast_compiler.bp†L80-L83】
- **Memory allocation failures.** Allocation failures for either the stored path or the module
  contents return early without diagnostics, leaving the host with an empty detail string.
  【F:compiler/ast_compiler.bp†L83-L104】

### `compileFromPath`

- **Null module path pointer.** A zero or negative path pointer aborts compilation without recording
  why the input was rejected. 【F:compiler/ast_compiler.bp†L112-L115】
- **Empty module path.** The path length check returns `-1` when `string_length(path_ptr)` is zero,
  and today the branch omits failure detail text. 【F:compiler/ast_compiler.bp†L117-L120】
- **Invalid cached module entry.** When the cached entry lacks a valid content pointer or length, the
  wrapper exits early without writing to the diagnostic buffer. 【F:compiler/ast_compiler.bp†L130-L134】
- **Downstream pipeline status codes.** Any negative status propagated from `compile_impl`
  (parsing, constant evaluation, validation, metadata generation, or code emission) ultimately
  surfaces through the host without extra context when the callee omits a detail message.
  【F:compiler/ast_compiler.bp†L142-L156】
- **Memory reservation failures now emit detail.** The linear-memory reservation branch writes a
  message today, so no additional work is required for this case. 【F:compiler/ast_compiler.bp†L126-L139】

## Stage1 pipeline phases lacking guaranteed diagnostics

`compile_impl` forwards negative statuses from each compilation phase. Several callees still return
`-1` without guaranteeing that a detail string was written first, producing silent failures in the
host wrapper.

- **Parsing failures.** `parse_program` returns a non-positive function count for syntax issues, but
  a number of parse errors still return `0` without emitting a message. 【F:compiler/ast_compiler.bp†L21-L32】【F:compiler/ast_parser.bp†L257-L360】
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
