# Stage2 Bootstrap Failure When Using `write_ascii_literal`

While investigating a colleague's report about stage2 bootstrap failures, I reproduced the issue by
changing the compiler sources to call `write_ascii_literal` with a string literal directly. The build
crashed during `bun ./src/cli.ts` with a stage2 compilation error that reported no diagnostic detail.

```
bun ./src/cli.ts
# …
error: stage2 compilation failed (status -1, functions=0, instr_offset=0, compiled_functions=0)
```

The root cause is the helper's signature:

```
fn write_ascii_literal(base: i32, offset: i32, literal: [u8; 32]) -> i32
```

Because the `literal` parameter is declared as a fixed `[u8; 32]` array, Bootstrap treats every string
literal argument as `[u8; <length>]`. Passing `"Undefined identifier \""` (length 22) does not satisfy the
function's type requirement, so constant evaluation fails during stage2 bootstrapping. The compiler exits
before it can emit a richer diagnostic, leaving hosts with the generic `stage2 compilation failed` error.

To avoid the failure today, allocate a 32-byte array for each diagnostic message and store the literal
into it before calling `write_ascii_literal`:

```
let prefix: [u8; 32] = "Undefined identifier \"\0\0\0\0\0\0\0\0\0\0";
diag_offset = write_ascii_literal(out_ptr, diag_offset, prefix);
```

Longer-term we should consider loosening the helper signature (for example by accepting slices) so that
callers can pass string literals directly.

# Stage2 Bootstrap Failure When Exhausting Call-Data Capacity

While adding additional diagnostics to the compiler we also ran into a different bootstrap failure
mode: once the semantic pass needs to allocate more scratch call-data than the arena reserves, the
stage2 build aborts without emitting a specific message. The arena size is controlled by the
`AST_CALL_DATA_CAPACITY` constant in `ast_compiler_base.bp`, which now reserves
`131072 - AST_CONSTANTS_SECTION_WORDS` four-byte slots (roughly doubling the arena
from its earlier size).【F:compiler/ast_compiler_base.bp†L2339-L2369】

Every diagnostic string copied through `write_diagnostic_literal` consumes some of that call-data
buffer. When the compiler tries to allocate storage for one more string than the capacity allows,
`ast_call_data_alloc` returns `-1`, the bootstrap falls back to the generic
`stage2 compilation failed` message, and no detail string is written.【F:compiler/ast_compiler_base.bp†L2448-L2459】

If you hit this error while introducing new diagnostics there are two options today:

* Increase `AST_CALL_DATA_CAPACITY` (and rebuild the compiler) so the arena reserves more space for
  string data.
* Remove or consolidate existing diagnostics so that the call-data usage stays within the current
  budget.

Either mitigation lets the stage2 bootstrap succeed again.
