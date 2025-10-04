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
  *Status:* Added scratch-backed AST storage with helpers for constructing typed arithmetic, comparison, and equality nodes, then extended the wrapper parser to lower those boolean-producing trees—including short-circuiting `&&`/`||` chains—before falling back to the legacy lowering path for unsupported constructs. Expanded that path to normalize boolean literals, locals, and unary `!` so pure-boolean expressions now stay within the AST pipeline.【F:compiler/stage1.bp†L153-L1559】【F:compiler/stage1.bp†L422-L596】【F:compiler/stage1.bp†L1345-L1509】

- [ ] **Convert user function calls to AST lowering**
  The simple-expression AST parser immediately returns `-3` whenever it sees a parenthesized identifier that is not recognized as an intrinsic, which forces `parse_expression` to reset its scratch space and fall back to the legacy lowering code for every user-defined call.【F:compiler/stage1.bp†L534-L681】 The old path then re-parses the call, performs argument arity/type checks, and emits the `call` instruction directly, so the AST pipeline never covers this common case.【F:compiler/stage1.bp†L4680-L4799】
  Steps to migrate calls into the AST pipeline:
  1. Introduce a new AST node kind that can capture the callee function index plus a variable number of child operands (arguments), and make sure `ast_allocate_node`/scratch bookkeeping can reserve space for the argument list metadata.【F:compiler/stage1.bp†L160-L216】【F:compiler/stage1.bp†L173-L213】
  2. Extend `parse_simple_expression_ast` to look up non-intrinsic callees via `functions_find`, build ASTs for each argument using the existing recursive helpers, and populate the new call node instead of returning `-3`. The node should record the signature’s result type so `parse_expression` can preserve type information.【F:compiler/stage1.bp†L534-L681】【F:compiler/stage1.bp†L4680-L4799】
  3. Teach `lower_simple_ast_node` to recognize the call node, emit any argument subtrees in order, and finish by writing an `emit_call` instruction that targets the recorded function index while restoring the scratch instruction offset on failure like the existing cases.【F:compiler/stage1.bp†L1743-L1816】【F:compiler/stage1.bp†L4680-L4799】
  4. Mirror the legacy path’s parameter validation in the AST branch so that argument count and type mismatches surface as AST parsing failures rather than forcing another fallback, keeping diagnostics consistent between the two pipelines.【F:compiler/stage1.bp†L4680-L4775】
