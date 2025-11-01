# Trait Metadata as `const` Values

## Background and Motivation
Traits already appear in the language concept as a foundational abstraction for operator overloading and shared behaviour, but the design document does not yet explain how they participate in the "types as const values" story that underpins generics.【F:concept.md†L6-L32】 Recent proposals established that structs can be materialised entirely in the const evaluator, yet trait metadata still depends on ad-hoc compiler definitions. To unlock trait-driven metaprogramming, we need intrinsics that let const functions manufacture trait descriptors and attach implementations to concrete types.

Two real-world patterns motivate this addition:

* **Operator traits stored in consts.** Developers should be able to define `const Addable = trait(...)` and then reuse the trait handle when assembling new types or forwarding methods, mirroring how `const Pair = struct(...)` enables flexible struct reuse.
* **Const trait factories.** Generic utilities benefit from synthesising trait constraints based on other const parameters, e.g. constructing a family of `prefixed(ELEM_TY)` traits that capture array behaviour for any element type.

## Proposed Intrinsics
Introduce two const-evaluable intrinsics that mirror the existing struct constructor:

```rust
const fn trait(
    const PROP_LEN: i32,
    const PROP_COUNT: i32,
    const PROPS: fn(type) -> [([u8; PROP_LEN], type); PROP_COUNT],
) -> trait

const fn impl(
    const TRAIT: trait,
    const TYPE: type,
    const PROPS: TRAIT(TYPE),
) -> implementation
```

Key semantics:

1. **Compile-time trait handles.** The `trait` intrinsic executes inside the const evaluator and yields a first-class trait handle. This handle can be bound to a `const`, returned from a const function, or supplied as a const argument alongside other type-level values.
2. **Property table factory.** `PROPS` is itself a const function from `type` to a fixed-length array of `(name, signature)` pairs. When evaluated, it receives the "Self" type and returns the canonical method set. Allowing a function parameter (instead of an eager array) ensures the same trait description can be reused for multiple `Self` types while still producing `type`-specialised signatures.
3. **Implementation attachment.** The `impl` intrinsic associates a `TRAIT` handle with a concrete `TYPE`. It verifies that the supplied property array matches the canonical property list produced by `TRAIT(TYPE)`, emitting an `implementation` value that can be combined with `TYPE` using existing `type + implementation` syntax.
4. **Identity and caching.** Two invocations of `trait` produce distinct trait handles unless all parameters—including the function body that builds the property array—are identical. This matches struct identity rules and ensures separate const factories yield unique trait metadata.
5. **Const safety.** Both intrinsics run before code generation, so any trait or implementation misuse can surface as compile-time diagnostics (duplicate property names, signature mismatches, missing methods, etc.).

## Language Surface
With the intrinsics in place, users gain the following capabilities:

* **Const trait aliases:**
  ```rust
  const Addable = trait(3, 1, fn(Self: type) -> [([u8; 3], type); 1] {[
      ("sum", fn(Self, Self) -> Self),
  ]});
  ```
  The resulting `Addable` handle can be referenced anywhere a `const trait` value is expected, mirroring how structs are reused via const bindings.【F:docs/proposals/trait-const-intrinsic.md†L38-L42】
* **Implementation packages:**
  ```rust
  const AddableI32Impl = impl(Addable, i32, [
      ("sum", fn(self: i32, other: i32) -> i32 { self + other }),
  ]);

  const AddableI32 = i32 + AddableI32Impl;
  ```
  The `impl` result merges with the base `type` to produce a trait-constrained type alias that carries the appropriate method surface.【F:docs/proposals/trait-const-intrinsic.md†L44-L50】
* **Const trait factories:**
  ```rust
  const fn prefixed(const ELEM_TY: type) -> trait {
      trait(6, 1, fn(Self: type) -> [([u8; 6], type); 1] {[
          ("prefix", fn(Self, const COUNT: i32) -> [ELEM_TY; COUNT]),
      ]})
  }
  ```
  Higher-order const functions can return trait handles that capture additional const parameters, letting trait definitions depend on other compile-time data.【F:docs/proposals/trait-const-intrinsic.md†L52-L58】
* **Type constructors with bundled traits:**
  ```rust
  fn prefixable_array(const ELEM_TY: type, const ELEM_COUNT: i32)
      -> [ELEM_TY; ELEM_COUNT] + prefixed(ELEM_TY) {
      [ELEM_TY; ELEM_COUNT] + impl(prefixed(ELEM_TY), [ELEM_TY; ELEM_COUNT], [
          ("prefix", fn(self: [ELEM_TY; ELEM_COUNT], const COUNT: i32) -> [ELEM_TY; COUNT] {
              let mut result = [ELEM_TY; COUNT];
              let mut idx = 0;
              while idx < COUNT {
                  result[idx] = self[idx];
                  idx = idx + 1;
              }
              result
          }),
      ])
  }
  ```
  This pattern mirrors existing struct const factories, enabling complex types to ship with ready-to-use trait implementations.【F:docs/proposals/trait-const-intrinsic.md†L60-L75】

## Implementation Notes
1. **Parser:** Extend syntax so const expressions can reference `trait` and `impl` intrinsics, and ensure `type + implementation` remains valid in const positions for aliasing trait-bearing types.
2. **Const evaluator:** Allocate trait descriptors, invoke the property factory with the supplied `Self` type, and store the resulting method table. For `impl`, validate the method list against the descriptor and produce an implementation handle.
3. **Type checker:** Track trait bounds on `type` expressions, permit `const T: type + SomeTrait` parameters, and verify method lookups (both UFCS-style `T::method` and value method calls `value.method(...)`) resolve through the attached implementation.
4. **Emitter:** Serialize trait metadata alongside struct/tuple records so Wasm/WGSL backends can dispatch methods, monomorphise trait calls, or issue static errors when required implementations are missing.
5. **Diagnostics:** Surface precise messages for duplicate method names, mismatched signatures, or attempts to attach implementations to incompatible types. Provide context that points back to the const intrinsic invocation sites to aid debugging.

Together, these intrinsics generalise the "types as const values" model to trait metadata, giving users fine-grained control over behaviour composition without introducing separate trait-definition syntax outside const evaluation.
