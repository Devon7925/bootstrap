
# TODO

- [ ] **Add target selection for CPU vs GPU compilation**
  The concept expects both WebAssembly and WGSL backends, but the current public API only emits Wasm and the CLI exposes no way to choose other targets. Introducing a `Target` enum and a `--target` flag would set the stage for a GPU path while continuing to default to Wasm.
  *Reference:* Concept target list【F:concept.md†L3-L14】, current Wasm-only API and CLI【F:src/lib.rs†L13-L97】【F:src/main.rs†L85-L197】

- [ ] **Support `f32` types in stage1**
  Floating-point numbers are part of the planned type system, yet stage1 currently hard-codes only `i32`, `bool`, and unit type codes. Extending the parser, type checker, and code generator to accept `f32` parameters, locals, literals, and arithmetic would close that gap and unblock future GPU math.
  *Reference:* Planned numeric types【F:concept.md†L22-L25】, existing stage1 type codes【F:compiler/stage1.bp†L580-L598】

- [ ] **Lower string literals to stack-allocated `u8` arrays**
  The concept specifies that string constants should become local byte arrays, but stage1 currently lacks any parsing for quoted literals. Teaching the tokenizer and expression lowering to recognize strings and materialize them as `u8` arrays would align the implementation with the design.
  *Reference:* String constant rule【F:concept.md†L31-L31】, absence of string literal handling in stage1 (no quoted literal parsing)【258989†L1-L2】

- [x] **Implement `load_u16`/`store_u16` intrinsics**
  Unsigned 16-bit integers are part of the language plan, yet the intrinsic table only exposes byte and 32-bit loads/stores. Adding 16-bit variants plus Wasm codegen would make it easier to model packed memory layouts.
  *Reference:* Numeric type requirements【F:concept.md†L22-L24】, current intrinsic coverage【F:compiler/stage1.bp†L247-L330】

- [ ] **Parse and register `type` definitions in stage1**
  Treating types as constant values requires a place to declare them, but the stage1 compiler presently scans only `fn` items. Allowing `type` aliases (e.g., `type Bytes = Array<u8, N>;`) to be recorded before function compilation would start building the infrastructure for type-level programming without affecting existing codegen.
  *Reference:* Type-as-constant goal【F:concept.md†L10-L13】, function-only parser loop【F:compiler/stage1.bp†L4580-L4619】
