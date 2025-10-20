# Type Template Investigation Notes

## Context

Attempting to extend `compiler/ast_compiler_base.bp` and `compiler/ast_parser.bp` to thread type-template metadata triggered stage2 failures when compiling even trivial programs (e.g. `fn main() -> i32 { 0 }`). The failure surfaced as `stage2 compilation failed (status -1, functions=0, instr_offset=0, compiled_functions=0)` with no additional detail.

## Findings

* Adding helper functions that load template handles from the call-data arena caused the compiler to abort before emitting any functions. The breakage reproduced even when the new helpers were not yet wired into the rest of the parser, suggesting the issue lies in the helper definitions themselves rather than downstream logic.
* The failure occurred regardless of whether the helpers were defined near the top of `ast_compiler_base.bp` or appended later in the file, and it reproduced with minimal bodies that simply read `load_i32(payload_ptr + slot * WORD_SIZE)`. Returning constant values or omitting the `load_i32` call avoided the failure.
* Adjusting `parse_function` to stage template-handle tables before const mask tables also caused the same stage2 error, implying the scratch-buffer layout is extremely sensitive and may need additional headroom or coordination with other temporary regions.

## Root Cause

Stage2’s memory layout only reserves space for 32,768 expression entries even though `AST_EXPR_CAPACITY` advertises support for 131,072 expressions. The expression arena begins at `ast_expr_count_ptr(ast_base)` and is expected to grow by `AST_EXPR_ENTRY_SIZE` (16 bytes) per expression. However, `ast_expr_types_base(ast_base)` advanced the base pointer by a fixed `524292` bytes, which was exactly `WORD_SIZE + 32_768 * AST_EXPR_ENTRY_SIZE`. Once the parser produced more than 32,768 expressions—something the new helpers do by introducing additional call expressions—the expression table overflowed into the adjacent expression-type metadata, corrupting the AST and causing the stage2 backend to abort before emitting any code.

## Resolution

* Expanded the expression arena to match the declared capacity (131,072 entries) by updating the offsets used by `ast_expr_types_base` and the scratch buffer, preventing expression metadata from overlapping adjacent sections while staying within stage1’s fixed 4 MB memory layout.【F:compiler/ast_compiler_base.bp†L6758-L6785】
* Added a regression test that boots the stage1 compiler inside the test harness, recompiles the compiler sources, and asserts the recorded expression count exceeds 65,536 to guard against future regressions.【F:test/regressions.test.ts†L1-L28】

## Verification

* `bun test`
