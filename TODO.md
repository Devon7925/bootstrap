# TODO

- [ ] **Support `f32` types in stage1**
  Floating-point numbers are part of the planned type system, yet stage1 currently hard-codes only `i32`, `bool`, and unit type codes. Extending the parser, type checker, and code generator to accept `f32` parameters, locals, literals, and arithmetic would close that gap and unblock future GPU math.
  *Reference:* Planned numeric types【F:concept.md†L22-L25】, existing stage1 type codes【F:compiler/stage1.bp†L580-L598】

- [ ] **Lower string literals to stack-allocated `u8` arrays**
  The concept specifies that string constants should become local byte arrays, but stage1 currently lacks any parsing for quoted literals. Teaching the tokenizer and expression lowering to recognize strings and materialize them as `u8` arrays would align the implementation with the design.
  *Reference:* String constant rule【F:concept.md†L31-L31】, absence of string literal handling in stage1 (no quoted literal parsing)【258989†L1-L2】

- [ ] **Parse and register `type` definitions in stage1**
  Treating types as constant values requires a place to declare them, but the stage1 compiler presently scans only `fn` items. Allowing `type` aliases (e.g., `type Bytes = Array<u8, N>;`) to be recorded before function compilation would start building the infrastructure for type-level programming without affecting existing codegen.
  *Reference:* Type-as-constant goal【F:concept.md†L10-L13】, function-only parser loop【F:compiler/stage1.bp†L4580-L4619】

- [ ] **Factor stage1 parsing into reusable helpers**
  The parser in stage1 inlines keyword matching, delimiter handling, and whitespace skipping at every call site, making the 5k+ line file hard to follow and extend. Extracting reusable helpers for repeated loops (parameter lists, return types, block scanning) would shrink the `compile` pipeline and clarify control flow.
  *Reference:* Repeated ad-hoc parsing logic inside `compile`【F:compiler/stage1.bp†L4560-L4662】

- [ ] **Centralize stage1 memory layout constants**
  The `compile` routine hand-computes offsets like `out_ptr + 4096` and `instr_base + instr_capacity - 12` each time, obscuring the intent of the scratch buffer layout. Introducing named constants or a small struct to manage these regions would make the code self-documenting and reduce arithmetic mistakes when the layout changes.
  *Reference:* Hard-coded scratch buffer offsets in `compile`【F:compiler/stage1.bp†L4665-L4679】
