# Proposal: Function Arguments as Const Parameters

## Summary

Extend const parameter specialization so that functions may be passed as compile-time arguments. The compiler should accept
both named functions and function-typed constants for const parameters, specialize call sites based on the supplied function
identity, and rewrite the specialized body so that the function argument is replaced with a concrete callable reference.
This complements the existing ability to specialize on integers, booleans, and types while unlocking metaprogramming patterns
that rely on higher-order behaviour.

## Motivation

The current const parameter design focuses on scalar and type arguments. Many language patterns benefit from supplying a
function at compile time:

* Choosing between multiple helper routines without incurring dynamic dispatch.
* Building pipelines where individual stages are fixed at compile time.
* Passing small adapters (for example, projection or comparison callbacks) into generic algorithms.

Today authors must emulate these behaviours by branching on integers or booleans and manually wiring the desired helper
inside each specialization, which quickly becomes brittle. Allowing const parameters to accept function references keeps the
existing `fn` syntax and lets authors express higher-order variants directly. The const specialization machinery already clones
functions per argument vector, so extending it to understand callable values follows the existing architecture.【F:docs/proposals/const-parameters.md†L49-L143】

## Detailed Design

### Function Types as Const Arguments

* Introduce a `fn(<params>) -> <return>` type expression that can appear wherever a type annotation is expected.
* A parameter annotated with `const CALLBACK: fn(i32) -> i32` requires callers to supply a compile-time known function whose
  signature matches the annotation. Signatures participate in type equality when selecting or reusing specializations.
* Function arguments may reference:
  * Top-level function declarations.
  * `const fn` definitions (the const evaluator produces a callable handle that can be reused at runtime).
  * `const` bindings whose value is a function.
  * Nested functions declared in the same module once those are supported.
* Function-typed const parameters are immutable just like other parameters. Within the specialized body, the compiler replaces
  occurrences of the parameter with a direct reference to the concrete function, eliminating any runtime indirection.

### Equality and Canonicalization

* The specialization cache keys function arguments by their fully qualified name plus the module identifier to avoid collisions
  across imports.【F:docs/proposals/const-parameters.md†L186-L207】
* Passing the same function from multiple call sites reuses the existing specialization. Distinct functions (even when they share
  identical signatures) produce unique clones so that inlining and constant folding can occur independently.
* The const evaluator resolves function references before executing a `const fn` that depends on them, ensuring compile-time
  execution has access to the specialized body. This mirrors how scalar const parameters are handled today.【F:docs/proposals/const-parameters.md†L151-L184】

### Weird Signatures and Higher-Order Composition

* Function types may themselves contain const parameters or return types that depend on const arguments. For example,
  `fn(const COUNT: i32, value: [i32; COUNT]) -> i32` is a valid function type for a const parameter.
* Callbacks with tuple arguments, array returns, or mixed scalar and type parameters are valid. The specialization pipeline
  rewrites the final function to call the supplied helper with the same calling convention it would have in ordinary code.
* Recursive function references remain unsupported as const arguments until recursive const evaluation is defined in a separate
  proposal; the compiler should emit a clear diagnostic when encountering them.

### Const and Let Bindings

* Top-level `const` bindings may store function references and can be passed to matching const parameters.
* Local `const` bindings inside function bodies are also eligible once their initializer is a compile-time resolvable function.
* Ordinary `let` bindings continue to be treated as runtime values; when a `let` binding referencing a function is supplied to a
  function-typed const parameter, the compiler reports that the argument is not a compile-time constant.

## Implementation Notes

1. **Parser** – Extend parameter parsing to recognize `fn` type annotations and capture whether a const parameter expects a
   callable. Update the AST metadata so that call sites record the referenced function identifier during constant folding.
2. **Semantic Analysis** – When checking const arguments, resolve function names to their definition handles and ensure the
   signature matches the annotated function type. Emit diagnostics for mismatches or attempts to pass runtime values.
3. **Specialization Pass** – Replace the function-typed const parameter node with the concrete callee during cloning. The pass
   should also rewrite call expressions so that they invoke the resolved function directly, skipping the placeholder parameter.
4. **Code Generation** – Because the specialized body now contains direct calls, no emitter changes are required beyond ensuring
   that function handles used in const contexts remain discoverable during Wasm lowering.
5. **Tooling** – Update error messaging helpers to include friendly descriptions of function types when explaining mismatches or
   non-constant arguments.

## Open Questions

1. Should the compiler permit function literals (closures or `fn` expressions) in const contexts once they exist, or restrict the
   feature to named functions and const bindings until closure lowering is implemented?
2. How should diagnostics render nested function types that themselves include const parameters to keep messages approachable?
3. Can specialization reuse be extended across modules for identical function pointers without violating encapsulation of private
   helpers?

## Future Work

* Allow `const fn` bodies to generate function tables that can be consumed by const parameters, enabling compile-time selection
  from large, programmatically constructed dispatch sets.
* Investigate whether the Wasm backend should inline short callbacks automatically once they are resolved by specialization.
* Explore exposing metadata about const-supplied functions to future traits or reflection systems for richer metaprogramming.
