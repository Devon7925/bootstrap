# Proposal: `while` Loops for Structured Control Flow

## Summary

Introduce first-class `while` loops to the language so authors can express conditional iteration without manually lowering to `loop` plus `if`/`break` constructs. The feature desugars a familiar Rust-style `while condition { body }` form into the existing unbounded loop representation, preserving the language's minimal runtime model while significantly improving ergonomics.

## Motivation

The language aspires to provide a Rust-inspired experience, including familiar syntax and control-flow constructs.【F:concept.md†L3-L9】 At present, the compiler intentionally rejects `while` statements with a hard error, forcing developers to spell every conditional loop using the lower-level `loop` primitive and explicit `if`/`break` checks.【F:test/control_flow.test.ts†L174-L199】 This gap has several downsides:

* **Readability:** Idiomatic control flow in reference materials, concept docs, and prior art relies on `while`. Requiring `loop` boilerplate makes simple iteration harder to read and review.
* **Onboarding:** Developers prototyping stage2 features instinctively reach for `while` and immediately hit an unsupported feature error, interrupting feedback cycles.
* **Code generation clarity:** Many existing tests simulate `while` by hand, obscuring the compiler's intent and complicating future optimizations that might recognize structured loops.

Adding a direct `while` surface form keeps the language aligned with the stated Rust inspiration while avoiding the need for authors to rediscover the manual lowering pattern each time.

## Current Status

Stage1 AST parsing detects the `while` keyword but aborts compilation with the "while statements are not supported" diagnostic.【F:test/control_flow.test.ts†L174-L183】 There is no downstream semantic or Wasm support because the construct never reaches those phases. All loop-related coverage in the test suite therefore exercises explicit `loop` blocks.

## Proposed Design

`while CONDITION { BODY }` behaves as a syntactic sugar that lowers to the already-supported `loop { if CONDITION { BODY_CONTINUE } else { break; } }` structure during AST construction. The lowering must:

1. Evaluate `CONDITION` before each iteration, enforcing that it produces a `bool` (matching existing truthy semantics for `if`).
2. Re-run the condition after `continue` statements in `BODY` without duplicating user code.
3. Preserve break values by mapping `break EXPR;` inside `BODY` directly to the enclosing loop's `break`.

By lowering early, later compilation stages continue to reason about a single loop representation, avoiding invasive changes to stage2.

## Implementation Outline

1. **AST Parser:** Replace the current rejection logic with parsing of `while` headers and bodies. Emit a dedicated AST node that carries pointers to the condition and body expressions.
2. **AST Lowering:** During block compilation, desugar each `while` node into a canonical loop-with-break form, allocating temporaries as necessary to preserve evaluation order.
3. **Semantic Checks:** Reuse existing `if` condition analysis to enforce boolean (or truthy) conditions and ensure the desugared loop inherits correct divergence tracking.
4. **Code Generation:** Because lowering reuses the existing loop representation, no new Wasm opcodes are required. Ensure the generated structure re-evaluates the condition after every iteration and `continue`.
5. **Diagnostics:** Update error reporting to point at the original `while` keyword when a condition has the wrong type or the body diverges unexpectedly.

## Difficulty Assessment

The work is **moderate** in scope. Parsing and desugaring require touching core compiler modules but leverage established patterns used for `if` expressions and loops. No new runtime intrinsics or Wasm instructions are necessary, and both stage1 and stage2 can continue sharing the lowered representation. Careful handling of source spans for diagnostics and ensuring correct interaction with `break`/`continue` are the trickiest aspects.

## Testing Strategy

* Positive execution tests covering simple counting loops, nested `while` loops, and `continue`/`break` usage.
* Negative tests ensuring non-boolean conditions are rejected and that `break value;` remains illegal for `while` (mirroring current semantics until optional value-carrying support is added).
* Cross-checks that the AST compiler and stage1 bootstrap both accept representative programs using `while`.

These cases should be captured as `test.todo` entries until the feature lands, then converted into active assertions.

## Risks and Open Questions

* **Condition Temporaries:** If a condition includes function calls with side effects, the lowering must re-evaluate it each iteration without accidentally hoisting evaluations. The design should document that behavior explicitly.
* **Future Extensions:** Supporting `while let`-style pattern loops or value-carrying `break` expressions may require revisiting the lowering strategy but can build on the foundational support proposed here.

By implementing `while`, we close a prominent ergonomics gap while keeping the compiler architecture focused on a single loop IR.
