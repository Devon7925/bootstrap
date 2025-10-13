# Proposal: `const` Parameters for Compile-Time Function Specialization

## Summary

Introduce support for annotating individual function parameters with the `const` keyword to indicate that the argument must be a compile-time constant. Const parameters allow authors to model many use cases that typically require generic type parameters (such as specializing on array lengths or literal values) while preserving the language's existing function declaration syntax.

```bootstrap
fn repeat(const COUNT: i32, value: i32) -> [i32; COUNT] {
    [value; COUNT]
}

fn main() -> i32 {
    let trio: [i32; 3] = repeat(3, 14);
    trio[0] + trio[1] + trio[2]
}
```

In the example above, `repeat` can be called with any constant `COUNT` without requiring a separate generic declaration form.

## Motivation

The language already treats types as constant values and supports `const fn` declarations that evaluate during compilation, as demonstrated throughout the existing test suite. However, functions currently lack an ergonomic way to accept compile-time values while remaining usable from non-const contexts. Today, authors must choose between:

* Hard-coding literal values inside the function body.
* Introducing top-level `const` definitions that mirror function parameters.
* Duplicating nearly identical functions when a value such as an array length needs to vary at compile time.

Adding `const` parameters lets a single function be specialized by the compiler based on constant arguments, avoiding boilerplate while staying aligned with the language's emphasis on compile-time evaluation.

## Detailed Design

### Syntax

A const parameter is declared by prefixing the parameter name with the `const` keyword inside an ordinary function signature.

```bootstrap
fn fill(const N: i32, value: i32) -> [i32; N] {
    [value; N]
}
```

* The parameter name still requires an explicit type annotation, matching the existing `name: Type` syntax used across the compiler and tests.
* Const parameters are available on both runtime (`fn`) and compile-time (`const fn`) functions. When used on a `const fn`, the parameter value is known during evaluation just like any other argument, but the const annotation enforces that callers supply a compile-time constant.
* Parameter ordering is unrestricted: const and non-const parameters can be interleaved freely.

### Call Semantics

* Arguments passed to const parameters must be expressions that the compiler already accepts as constant in other contexts (literals, references to `const` values, and `const fn` calls that satisfy evaluation rules).
* At call sites, the compiler resolves const arguments during stage 1, before emitting Wasm or WGSL, mirroring existing constant evaluation paths.
* Each distinct set of const argument values yields a separately specialized function instance in the generated code. Non-const arguments continue to behave identically across specializations.

```bootstrap
const fn width() -> i32 {
    4
}

fn scale(const FACTOR: i32, value: i32) -> i32 {
    value * FACTOR
}

fn main() -> i32 {
    let four = scale(width(), 10); // OK: `width()` is a const fn call.
    let five = scale(5, 10);
    four + five
}
```

### Type System Interaction

Const parameters act as immutable compile-time values. Within the function body they behave like `const` bindings:

* They cannot be reassigned (`FACTOR = 2` is rejected), matching the immutability of standard parameters.
* They can appear in type expressions (e.g., `[i32; FACTOR]`), enabling array and tuple lengths or other constant-driven types.
* Parameter type annotations can depend on const parameters declared earlier in the signature, letting a single declaration specialize both the parameter and return types for each constant argument.
* They participate in constant folding and dead-code elimination just like other constant expressions.

### Name Resolution

Const parameters share the same scope and shadowing rules as existing parameters. The following is valid and mirrors non-const behavior:

```bootstrap
fn example(const VALUE: i32) -> i32 {
    let VALUE: i32 = VALUE + 1; // Local binding shadows const parameter.
    VALUE
}
```

## Examples

### Array Length Specialization

```bootstrap
fn zeroes(const LEN: i32) -> [i32; LEN] {
    [0; LEN]
}

fn main() -> i32 {
    let pair = zeroes(2);
    pair[0] + pair[1]
}
```

### Compile-Time Dispatch

```bootstrap
const fn cube(x: i32) -> i32 {
    x * x * x
}

fn dispatch(const USE_CUBE: bool, value: i32) -> i32 {
    if USE_CUBE {
        cube(value)
    } else {
        value * value
    }
}

fn main() -> i32 {
    dispatch(true, 3) + dispatch(false, 3)
}
```

### Mixing Const and Runtime Parameters

```bootstrap
fn repeat_value(value: i32, const TIMES: i32) -> i32 {
    let mut total: i32 = 0;
    let mut index: i32 = 0;
    loop {
        if index >= TIMES {
            return total;
        };
        total = total + value;
        index = index + 1;
        0
    }
}

fn main() -> i32 {
    repeat_value(6, 7)
}
```

### Parameter Type Specialization

```bootstrap
fn dot(const N: i32, lhs: [i32; N], rhs: [i32; N]) -> i32 {
    let mut sum: i32 = 0;
    let mut index: i32 = 0;
    loop {
        if index >= N {
            return sum;
        };
        sum = sum + lhs[index] * rhs[index];
        index = index + 1;
        0
    }
}

fn main() -> i32 {
    let lhs = [1, 2, 3, 4];
    let rhs = [5, 6, 7, 8];
    dot(4, lhs, rhs)
}
```

In `dot`, the compiler emits distinct parameter and return types for each `N`, allowing ergonomic APIs whose calling conventions depend on compile-time constants without requiring separate overloads.

## Implementation Notes

### Parser support

The current parser assumes every parameter appears as `name: Type` and records each identifier inside fixed-width tables during `parse_function`. Supporting `const` parameters requires detecting the keyword before the identifier, tracking which slots are compile-time only, and still storing the ordinary type information for downstream passes.【F:compiler/ast_parser.bp†L2157-L2284】 Identifier expressions presently lower parameter accesses into expression kind `6` with a positional index, so the specialization phase must replace any reads of `const` parameters with literal nodes or other constant-aware constructs to prevent those slots from leaking into the final AST.【F:compiler/ast_parser.bp†L878-L886】

### Specialization pipeline

The main compilation pipeline parses source, interprets global constants, validates semantics, and finally emits Wasm/WGSL.【F:compiler/ast_compiler.bp†L18-L47】 Because const parameter functions can also be marked `const fn`, the interpreter may need to execute them while evaluating top-level constants, so specialization cannot wait until after `interpret_constants` finishes. Instead, the pass should register template functions before constant evaluation begins and expose an API that the constant interpreter and later validation can call to request (or reuse) a specialization when they encounter a call whose arguments fold to constants.【F:compiler/ast_semantics.bp†L20-L119】【F:compiler/ast_compiler_base.bp†L3951-L4210】 This lets const evaluation obtain the specialized body needed for execution while still ensuring the post-specialization AST contains only ordinary parameter lists. The cache for each template function continues to be keyed by the canonicalized const argument vector; when a new combination is encountered, clone the template body, rewrite const uses, and append the specialized function to the AST while updating the global function count.

### Rewriting bodies and metadata

Because locals are indexed relative to the number of parameters (`params_count + offset`), removing const parameters requires renumbering both surviving parameters and every local declaration in the cloned body.【F:compiler/ast_compiler_base.bp†L785-L807】 Cloning also has to duplicate the tree of expression nodes stored in the flat AST arena using helpers such as `ast_expr_alloc`, so the pass needs a worklist that remaps old indices to newly allocated nodes without exceeding the arena capacity.【F:compiler/ast_compiler_base.bp†L3411-L3433】 When rewriting, convert each const parameter read into an evaluated literal (or, for type-valued consts, into the resolved type id) so that semantic validation observes concrete values.

### Call resolution and canonicalization

`resolve_call_metadata` currently matches calls by name and exact parameter count, enforcing type equality against the callee's stored signature.【F:compiler/ast_semantics.bp†L20-L119】 Specialization must update each call site's metadata to reference the concrete specialized function and trim const arguments from the runtime argument list before validation runs. The pass also needs to evaluate const arguments to canonical values (respecting integer widths, booleans, and type identifiers) so that the specialization key is stable; this aligns with the language goal of treating types as first-class constant values that can flow through const parameters.【F:concept.md†L1-L32】

### Code generation considerations

The Wasm emitter derives local allocations by walking expression trees and counting locals relative to the final parameter count, so specialized functions must expose the correct runtime signature and maintain consistent local indices after const parameters are removed.【F:compiler/wasm_output.bp†L418-L510】 Updating call metadata during specialization also avoids touching the emitter, because it already expects each call node to hold the callee index and argument expressions for a fully concrete function body. Template entries that retain const parameters should be excluded from the exported function list so that the embedding environment only sees runnable, fully specialized functions; otherwise the host could observe signatures containing const-only parameters that it cannot instantiate.【F:compiler/wasm_output.bp†L2257-L2305】 Because the compiler cannot specialize calls that originate from an embedding with unknown const values, host-visible APIs must use the generated specializations (or an adapter that dispatches among them) rather than the templates themselves.

### Implementation challenges

* **Template lifetime management:** The template function containing const parameters should remain available for future instantiations but must be skipped during emission to honor the requirement that const parameters disappear from the final AST. One option is to flag template entries so validation and emission ignore them while the specialization pass consults them when needed.
* **Const evaluation integration:** Constant interpretation invokes user-defined `const fn` bodies, so the specialization cache must be callable from the interpreter to provide the correct instantiation before evaluation proceeds.【F:compiler/ast_semantics.bp†L20-L119】【F:compiler/ast_compiler_base.bp†L3951-L4210】
* **Cross-module reuse:** When const functions are imported, the specialization cache has to incorporate the module index (or fully qualified name) so that identical const vectors in different modules still resolve to the correct definition.
* **Diagnostic clarity:** Because specialization happens before validation, error reporting for non-constant arguments or evaluation failures should surface at the call site with the evaluated argument context rather than deep inside the cloning logic.

## Alternatives Considered

* **Generic Type Parameters:** Traditional generics introduce additional syntax (angle brackets, explicit instantiation rules) and require a more complex type system. Const parameters offer a lighter-weight path that leverages the existing constant evaluation infrastructure.
* **Dedicated `const` Blocks or Traits:** Encapsulating compile-time configuration in traits or specialized blocks could address some use cases, but would not integrate as smoothly with the current function syntax.

## Open Questions

1. Should the compiler memoize const specializations to avoid code duplication when identical const arguments appear across modules?
2. How should error messages differentiate between non-const arguments and const arguments that fail to evaluate (e.g., due to recursion limits)?
3. Are there targets (such as WGSL) where excessive specialization may exceed existing stage limits, and if so, should tooling warn when const parameter usage approaches those constraints?

## Future Work

* Allow const parameters on type aliases and `type` declarations to express compile-time families of types without separate trait machinery.
* Explore allowing default values for const parameters to support ergonomic APIs when a common constant is frequently used.
* Investigate exposing const parameter values to metaprogramming facilities once they exist, enabling reflection over compile-time configuration.
