# TODO

- [ ] **Support `f32` types in stage1**
  Floating-point numbers are part of the planned type system, yet stage1 currently hard-codes only `i32`, `bool`, and unit type codes. Extending the parser, type checker, and code generator to accept `f32` parameters, locals, literals, and arithmetic would close that gap and unblock future GPU math.
  *Reference:* Planned numeric types【F:concept.md†L22-L25】, existing stage1 type codes【F:compiler/stage1.bp†L580-L598】

- [ ] **Lower string literals to stack-allocated `u8` arrays**
  The concept specifies that string constants should become local byte arrays, but stage1 currently lacks any parsing for quoted literals. Teaching the tokenizer and expression lowering to recognize strings and materialize them as `u8` arrays would align the implementation with the design.
  *Reference:* String constant rule【F:concept.md†L31-L31】, absence of string literal handling in stage1 (no quoted literal parsing)【258989†L1-L2】

- [x] **Factor stage1 parsing into reusable helpers**
  The parser in stage1 inlines keyword matching, delimiter handling, and whitespace skipping at every call site, making the 5k+ line file hard to follow and extend. Extracting reusable helpers for repeated loops (parameter lists, return types, block scanning) would shrink the `compile` pipeline and clarify control flow.
  *Reference:* Repeated ad-hoc parsing logic inside `compile`【F:compiler/stage1.bp†L4560-L4662】
  *Status:* Added dedicated helpers for parameter lists and optional return types so both signature registration and body compilation share the same parsing routines, removing duplicated loops and keyword handling.

- [x] **Introduce `SourceCursor` helpers for stage1 parser**
  Stage1 threads the `base`, `len`, and index triplet through nearly every parsing function, reapplying whitespace skipping and byte peeks manually. Creating a lightweight cursor struct with methods for advancing, peeking, and matching delimiters would reduce parameter lists and make control flow clearer.
  *Reference:* Parsing functions repeatedly pass `base`, `len`, and `idx` while chaining `skip_whitespace` and `expect_char` calls【F:compiler/stage1.bp†L4520-L4547】
  *Status:* Added reusable cursor storage in scratch memory with helpers for skipping, peeking, and keyword matching, then rewrote signature registration and the top-level compiler loop to drive parsing via that cursor instead of raw `base`/`len`/`idx` triples.【F:compiler/stage1.bp†L126-L238】【F:compiler/stage1.bp†L481-L707】【F:compiler/stage1.bp†L5256-L5545】

- [x] **Centralize keyword recognition logic**
  Detecting `type` and other keywords currently performs ad-hoc byte comparisons for each character before dispatching, leading to verbose and error-prone control flow. Providing a shared helper that validates reserved words and their boundaries would simplify loops that scan items during registration.
  *Reference:* Manual `type` keyword detection compares individual bytes before calling `expect_keyword_type`【F:compiler/stage1.bp†L4564-L4598】
