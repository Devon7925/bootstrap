# Proposal: Anonymous Function Expressions

## Summary

Add expression-level `fn` literals that evaluate to first-class function values. Anonymous functions share the same signature
syntax as named declarations and can be stored in constants, passed as const arguments, or returned from `const fn` factories
without requiring a top-level definition. Each literal produces a distinct callable that can participate in both runtime
dispatch and compile-time specialization, while respecting the existing rule that function values are only permitted in const
positions.【F:docs/proposals/const-parameter-function-values.md†L25-L74】

```bootstrap
const add_one = fn(x: i32) -> i32 {
    x + 1
};

fn apply_twice(const F: fn(i32) -> i32, value: i32) -> i32 {
    F(F(value))
}

fn main() -> i32 {
    apply_twice(10, add_one)
}
```

## Motivation

Bootstrap encourages a Rust-inspired programming model that supports higher-order code via function types and const
specialization.【F:concept.md†L3-L13】【F:docs/proposals/const-parameter-function-values.md†L1-L67】 Today, however, authors must
declare every helper function at the top level before they can be referenced, even when the helper is a short, single-use
operation. This friction shows up when:

* Constructing pipelines in const parameters where inlining a bespoke helper would avoid extra naming ceremony.【F:docs/proposals/const-parameter-function-values.md†L9-L54】
* Building configuration tables that naturally want to embed a per-entry function value without expanding each one into a global
  symbol.
* Returning callbacks from factories or adapters that depend on runtime arguments.

Introducing anonymous function expressions unlocks these patterns while preserving the explicit typing and deterministic code
generation model that the language already emphasizes. The feature aligns with the stated goal of making functions first-class
values without forcing every helper to occupy global scope.

## Detailed Design

### Syntax and Parsing

* An anonymous function literal uses the same signature syntax as a standard `fn` declaration: `fn(<params>) -> <return> { <body> }`.
* Parameter lists follow the existing rules for names, type annotations, and optional `const` modifiers. Default arguments remain
  unsupported, matching the current language status.
* The return type is mandatory unless the function's body is a block whose final expression has a known type; this mirrors the
  requirement for named functions and keeps inference consistent.
* Parsing produces a new AST node (e.g., `FunctionExpr`) that captures the parameter list, optional return type, and body block.
  The node also records a synthetic symbol name derived from the surrounding location for diagnostics and debugging.

### Type Checking and Inference

* Anonymous functions synthesize a `fn(<params>) -> <return>` function type during type checking, using the same canonicalization
  logic that named functions rely on.【F:docs/proposals/const-parameters.md†L151-L207】
* The literal's function type can flow through assignments, struct fields, and expression contexts. When the surrounding
  expression expects a specific function type, the compiler enforces signature compatibility and reports mismatches with the same
  diagnostics used for named functions.
* Parameter and return types must be explicitly written, mirroring the requirements for named functions. Contextual type
  information is used only to verify compatibility; omitting annotations continues to trigger the existing "missing type"
  diagnostics.
* Because function values are confined to const contexts, any parameter that uses a function type must itself be declared
  `const`. Returning a function is only permitted from a `const fn`, and the returned signature must consist solely of
  `const` parameters so the value can be materialized during compile time.【F:docs/proposals/const-parameter-function-values.md†L25-L74】

### Evaluation Semantics

* Each evaluation of an anonymous function literal allocates a fresh callable object. When the literal appears in a `const`
  context, the compiler emits a compile-time known function handle that can be reused by const parameters and specialization
  machinery.【F:docs/proposals/const-parameter-function-values.md†L23-L40】 Literals evaluated at runtime must immediately flow
  into a `const` binding or parameter; other contexts trigger the existing "functions are const-only" diagnostics.
* Anonymous functions are **non-capturing** in their initial form. They may reference global symbols, other functions, and `const`
  bindings, but they cannot close over `let` bindings or mutable state from the surrounding scope. The compiler rejects attempts
  to capture non-const locals with a diagnostic indicating that closures are not yet supported.
* Because they do not capture, anonymous functions can be safely hoisted during constant evaluation. The compiler treats them as
  equivalent to nested named functions that are immediately referenced, avoiding additional environment records.

### Interaction with Existing Features

* **Const Parameters:** Anonymous functions may be passed to function-typed const parameters, enabling inline specialization
  without top-level declarations. The specialization cache keys these literals using their synthesized identity so that repeated
  uses of the same literal within a module reuse the resulting clone.【F:docs/proposals/const-parameter-function-values.md†L41-L77】
  Non-const parameters continue to reject function types, so callers must supply anonymous literals through `const` arguments.
* **`const fn`:** A `const fn` can return an anonymous function, allowing compile-time factories that produce tailored helpers.
  The returned literal must itself be const-evaluable and expose only `const` parameters so that the resulting value can be used
  wherever other function constants are accepted.
* **Trait Implementations (Future):** When trait-style method tables arrive, anonymous functions can populate trait objects or
  adapters without requiring explicit free functions. Because literals have stable identities, they can be stored in compile-time
  trait metadata just like named functions.
* **Code Generation:** The Wasm and WGSL emitters treat anonymous function literals as regular function definitions. Each literal
  is assigned an internal function index, emitted alongside named functions, and referenced by handles in the generated code. No
  new runtime calling conventions are required.

## Examples

### Passing Inline Helpers to Const Parameters

```bootstrap
const fn square(x: i32) -> i32 {
    x * x
}

fn map_pair(const F: fn(i32) -> i32, lhs: i32, rhs: i32) -> (i32, i32) {
    (F(lhs), F(rhs))
}

fn main() -> i32 {
    let doubled = map_pair(fn(x: i32) -> i32 { x + x }, 4, 7);
    let squared = map_pair(square, 3, 5);
    doubled.0 + squared.1
}
```

### Returning an Anonymous Function

```bootstrap
const fn make_incrementer() -> fn(const x: i32) -> i32 {
    fn(const x: i32) -> i32 { x + 1 }
}
```

Returning an anonymous function requires the surrounding declaration to be `const fn`, and the returned signature must list
only `const` parameters. Anonymous functions remain non-capturing, so attempts to close over values like `delta` will continue
to error until closure support is introduced; see Future Work.

### Constructing Lookup Tables

```bootstrap
const HANDLERS: [fn(i32) -> i32; 2] = [
    fn(x: i32) -> i32 { x + 1 },
    fn(x: i32) -> i32 { x - 1 },
];

fn dispatch(idx: i32, value: i32) -> i32 {
    HANDLERS[idx](value)
}
```

## Implementation Notes

1. **Parser:** Extend the expression grammar to parse `fn` literals wherever expressions are permitted. Reuse the existing
   parameter-list parsing helpers from named functions to ensure consistent diagnostics.
2. **AST & IR:** Introduce a distinct node for anonymous functions and update the compiler's symbol tables to allocate synthetic
   identifiers. Ensure the lowering pipeline emits function definitions for these nodes before code generation begins.
3. **Type Checker:** Reuse existing function-signature checking to validate literals against expected contexts and emit capture
   errors when the body references non-const locals from the outer scope.
4. **Const Evaluator:** Treat anonymous function literals as constant-producible values. When evaluated in const contexts, intern
   the resulting function handle so repeated literals share identity when appropriate.
5. **Runtime Representation:** Update function tables or dispatch metadata to include anonymous literals. This may require
   reserving name strings for debug info but otherwise follows existing function emission.

## Open Questions

1. Should the compiler deduplicate structurally identical literals across modules, or is per-site specialization preferred to keep
   debug traces straightforward?
2. How should anonymous function names appear in diagnostics and stack traces? Options include synthetic labels (`"<anon@file:line>"`)
   or developer-provided labels via an attribute syntax.
3. Once capturing closures are introduced, how will captured variables be represented in Wasm/WGSL without violating the current
   memory model?

## Future Work

* Extend anonymous functions to support capturing locals, forming true closures that can encapsulate state across invocations.
* Investigate short closure syntax (e.g., `|x| x + 1`) built on top of the same lowering once anonymous `fn` literals are stable.
* Explore constexpr-friendly traits or interfaces that can be populated directly with anonymous functions for compile-time
  metaprogramming.
