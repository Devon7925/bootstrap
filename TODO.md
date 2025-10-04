# TODO

- [ ] **Support `f32` types in stage1**
  Floating-point numbers are part of the planned type system, yet stage1 currently hard-codes only `i32`, `bool`, and unit type codes. Extending the parser, type checker, and code generator to accept `f32` parameters, locals, literals, and arithmetic would close that gap and unblock future GPU math.
  *Reference:* Planned numeric types【F:concept.md†L22-L25】, existing stage1 type codes【F:compiler/stage1.bp†L580-L598】

- [ ] **Lower string literals to stack-allocated `u8` arrays**
  The concept specifies that string constants should become local byte arrays, but stage1 currently lacks any parsing for quoted literals. Teaching the tokenizer and expression lowering to recognize strings and materialize them as `u8` arrays would align the implementation with the design.
  *Reference:* String constant rule【F:concept.md†L31-L31】, absence of string literal handling in stage1 (no quoted literal parsing)【258989†L1-L2】

- [x] **Introduce `SourceCursor` helpers for stage1 parser**
  Stage1 threads the `base`, `len`, and index triplet through nearly every parsing function, reapplying whitespace skipping and byte peeks manually. Creating a lightweight cursor struct with methods for advancing, peeking, and matching delimiters would reduce parameter lists and make control flow clearer.
  *Reference:* Parsing functions repeatedly pass `base`, `len`, and `idx` while chaining `skip_whitespace` and `expect_char` calls【F:compiler/stage1.bp†L4520-L4547】
  *Status:* Added reusable cursor storage in scratch memory with helpers for skipping, peeking, and keyword matching, then rewrote signature registration and the top-level compiler loop to drive parsing via that cursor instead of raw `base`/`len`/`idx` triples.【F:compiler/stage1.bp†L126-L238】【F:compiler/stage1.bp†L481-L707】【F:compiler/stage1.bp†L5256-L5545】

- [ ] **Adopt `ParserContext` to bundle mutable parser state**
  Most stage1 routines pass `scope`, `arena`, and diagnostic sinks separately, cluttering signatures and increasing the chance of mismatched lifetimes. Wrapping these in a lightweight context struct with scoped accessors would clarify ownership and improve testability.
  *Reference:* Parser functions accept multiple loosely related parameters【F:compiler/stage1.bp†L4488-L4520】

- [ ] **Standardize diagnostic emission helpers**
  Error reporting interleaves message formatting with control flow, leading to inconsistent phrasing and missed context. Introducing shared helpers for span creation and message templating would keep compiler errors uniform while shrinking the amount of inline glue code.
  *Reference:* Manual diagnostic construction during signature parsing and type checking【F:compiler/stage1.bp†L4700-L4765】

- [ ] **Layer intermediate AST passes between parsing and lowering**
  Stage1 currently lowers directly from tokens to bytecode, which tangles syntax handling with code generation details. Introducing a lightweight AST normalization pass would isolate grammar concerns, simplify transformations, and make the compiler easier to reason about.
  *Reference:* Direct lowering from parser into bytecode emission routines【F:compiler/stage1.bp†L5000-L5180】
