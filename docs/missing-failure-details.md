# Gaps in Stage1 Failure Diagnostics

The stage1 toolchain reports structured failures through `error.failure.detail`, but a few error
paths never populate the diagnostic string. The situations below currently surface a
`Stage1CompileFailure` whose `failure.detail` field is empty.

## Host harness guardrails

- **Input buffer too small.** When the host tries to copy the source into memory past the
  configured input pointer, the harness throws before the compiler runs. The helper clears the
  failure buffer via `zeroFailureDetail`, so the resulting failure has no detail message. 【F:test/helpers.ts†L493-L517】【F:test/helpers.ts†L114-L122】
- **Output buffer too small.** The same harness guard runs when the reserved output region would
  overflow the linear memory, again after the diagnostic buffer has been zeroed. 【F:test/helpers.ts†L508-L517】【F:test/helpers.ts†L114-L122】
- **Advancing compilation cursors past memory.** `compileWithStride` mirrors the Rust harness by
  throwing if the next input or output cursor would exceed the allocated memory and relies on the
  generic failure wrapper, which surfaces without a detail string. 【F:test/helpers.ts†L549-L555】【F:test/helpers.ts†L587-L594】
- **Stage1 WebAssembly exports misbehaving.** Any exception, non-finite result, or non-positive byte
  count returned by the WebAssembly `compile` export triggers the generic `#failure` wrapper. If the
  compiler runtime did not write a message (for example, because execution stopped in native code),
  the reported failure lacks `detail`. 【F:test/helpers.ts†L521-L537】【F:test/helpers.ts†L587-L594】

## Module loading API fallbacks

- **`loadModuleFromSource` argument or storage failures.** The WebAssembly side returns `-1` for
  invalid pointers, empty paths, exceeding the module cache capacity, or allocation failures, but it
  never fills the failure buffer for those guard clauses. 【F:compiler/ast_compiler.bp†L63-L105】
- **`compileFromPath` rejects invalid input.** Several early exits—such as a null path pointer, zero
  length, missing module content, or a failed memory reservation—return `-1` without emitting a
  diagnostic. Only the "module has not been loaded" branch writes a detail string today. 【F:compiler/ast_compiler.bp†L111-L149】
- **Linear-memory growth failures.** If `ensure_memory_capacity` cannot satisfy the requested size,
  it simply returns `-1`, so the caller propagates a failure with an empty detail field. 【F:compiler/ast_compiler_base.bp†L18-L39】

## Compiler pipeline entrypoints

- **Downstream phases that return `< 0` without a diagnostic.** `compile_impl` simply forwards
  negative statuses from parsing, constant interpretation, validation, metadata emission, and code
  generation. When a callee forgets to write into the failure buffer, the host reports the failure
  without detail. 【F:compiler/ast_compiler.bp†L25-L53】【F:test/helpers.ts†L587-L594

These gaps highlight the guard rails that still need bespoke diagnostics so the host can surface
actionable error messages to users.
