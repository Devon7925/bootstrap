# Gaps in Stage1 Failure Diagnostics

The Stage1 toolchain reports structured failures through `error.failure.detail`, but several code
paths still return a `Stage1CompileFailure` without populating the diagnostic string. The situations
below currently drop detail text and should be tackled individually so fixes remain tractable.

## Module loading APIs

### `compileFromPath`

- **Memory reservation failures now emit detail.** The linear-memory reservation branch writes a
  message today, so no additional work is required for this case. 【F:compiler/ast_compiler.bp†L149-L154】

## Stage1 pipeline phases lacking guaranteed diagnostics

`compile_impl` forwards negative statuses from each compilation phase. Several callees still return
`-1` without guaranteeing that a detail string was written first, producing silent failures in the
host wrapper.

These gaps highlight the remaining guardrails that need bespoke diagnostics so the host can surface
actionable error messages to users.
