# Struct Types as `const` Values

## Background and Motivation
The language concept positions structs alongside tuples, enums, and arrays as fundamental data aggregates, and it explicitly calls out that "types can be treated as constant values" to unlock generic programming without a separate type-parameter syntax.【F:concept.md†L10-L21】 Existing tests already demonstrate this capability for tuples and arrays: const functions can return tuples containing both `type` and value entries, while arrays of `type` values can be indexed at compile time and used for later annotations.【F:test/const_type_value_intermixing.test.ts†L1-L86】 The ergonomics and metaprogramming opportunities unlocked by these patterns should extend to user-defined struct layouts as well.

## Current Progress
* The test suite exercises a placeholder `struct_like` `const fn` that matches the desired signature for a struct-construction intrinsic, but it simply returns `i32` today; no actual struct type is produced.【F:test/const_type_value_intermixing.test.ts†L137-L147】
* Stage1 compiler sources do not yet build structural metadata from const contexts, and there is no emission support for struct layout or field access. The Wasm emitter's `write_type_metadata` pass walks array and tuple registries exclusively, so struct layouts are never recorded today.【F:compiler/wasm_output.bp†L3271-L3291】
* Documentation lacks a dedicated proposal outlining how struct type construction and const evaluation should interact.

These gaps make it difficult to reason about how the compiler should eventually lower struct literals, member access, and dynamic struct construction driven by const computations.

## Proposed `struct` Intrinsic
Introduce a compiler intrinsic with the following signature:

```rust
const fn struct(
    const STR_LEN: i32,
    const PROP_COUNT: i32,
    const PROPS: [([u8; STR_LEN], type); PROP_COUNT],
) -> type
```

Key semantics:

1. **Compile-time evaluation:** The intrinsic executes entirely inside the const evaluator. The resulting `type` value becomes a first-class struct type that can be stored in const bindings, returned from const functions, or supplied as const parameters just like tuples or arrays of types.
2. **Property encoding:** Each property is expressed as a pair `([u8; STR_LEN], type)`. The UTF-8 bytes (terminated with `\0`) define the canonical field identifier; struct literals must supply labels whose bytes exactly match this canonical form after trimming trailing null padding.
3. **Deterministic layout:** Evaluation of `struct` records the order of entries from `PROPS`. Field offsets are computed during semantic analysis using the canonical field list, enabling both `.name` and `[const_name]` access syntax.
4. **Type identity:** Two `struct` invocations produce distinct types unless all parameters (including string contents and element types) are bitwise-identical. This mirrors tuple behaviour and permits dynamic struct factories inside const functions.
5. **Interop with arrays of types:** Because `PROPS` itself is an array that may be assembled programmatically, const functions can build structs whose property sets depend on runtime-independent computations (for example, iterating over digits to manufacture keys).

## Language Surface
The intrinsic unlocks the following user-facing behaviours:

* **Type aliases:** `const Pair = struct(6, 2, [("first\0", i32), ("second", i32)]);` defines a reusable struct type.
* **Instance construction:** Struct values use Rust-like literal syntax: `let value: Pair = Pair { first: 1, second: 2 };`. A bracket form `value[CONST_NAME]` offers dynamic field access when the key is supplied as a `[u8; N]` const array whose contents exactly match the canonical field name (ignoring trailing null padding).
* **Const factories:** Higher-order const functions can return struct types by manipulating arrays of `(name, type)` entries before calling `struct`, enabling patterns like `dynamic_struct(KEY_COUNT)` in the motivating example.

## Implementation Notes
1. **Parser:** Recognise struct literal syntax that permits both identifier labels (`field:`) and bracketed const labels (`[CONST]:`).
2. **Const evaluator:** Extend the evaluator to allocate `struct` type handles and store the field table (name bytes, type handles, computed offsets).
3. **Type checker:** Resolve dot and bracket access against the field table, verifying that bracket labels are `[u8; N]` const expressions and performing compile-time name matching.
4. **Emitter:** Emit Wasm (and later WGSL) metadata for struct layout, including field offsets, alignment, and constructor semantics analogous to tuple lowering.
5. **Diagnostics:** Surface helpful errors when property arrays have duplicate names, mismatched lengths, or invalid UTF-8, and when struct literals omit required fields or specify unknown ones.

This proposal complements the existing const-type story by making structs a first-class participant in compile-time metaprogramming. It keeps the language surface orthogonal—structs are introduced via a single intrinsic while sharing expression forms (literals, field access) with existing aggregates.
