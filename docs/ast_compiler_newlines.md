# `ast_compiler.bp` blank-line sensitivity investigation

## Reproduction summary
- Start from the tip of the repository and modify `compiler/ast_compiler.bp` by inserting blank lines inside `fn main() -> i32 { … }`.
- Two extra blank lines (file length +10 bytes) still allow the stage2 compiler to rebuild itself.
- Three blank lines (+15 bytes) consistently make stage2 abort while emitting the code section for the `is_whitespace` helper (function index 6). The failure report shows the instruction cursor contains the ASCII bytes `0x65 0x73 0x5f 0x62` ("es_b"), and function index 11 suddenly claims a body length of `148_636_160` bytes before stage2 stops with `compiled=12`.【F:compiler/ast_compiler.bp†L6533-L6576】【ae79d4†L1-L33】

## Why the third blank line matters
Stage2 relies on a handwritten linear-memory layout, defined directly in `compiler/ast_compiler.bp`. All offsets are computed from the caller-provided `out_ptr` (which our Rust host sets to `source.len()`):

- Scratch metadata (instruction cursor, temporary buffers, and the legacy stage1 function table) starts at `out_ptr + 4_096` and ranges up to `out_ptr + 851_968`.【F:compiler/ast_compiler.bp†L1949-L2038】
- The parsed AST is stored after `out_ptr + input_len + 65_536`. Within that region, the expression table begins at `out_ptr + (2·input_len + 401_424)` and consumes 16 bytes per expression node before spilling into the expression-type table.【F:compiler/ast_compiler.bp†L2057-L2089】【F:compiler/ast_compiler.bp†L2176-L2240】

For the current bootstrap source (`input_len = 208_894` and `expr_count = 16_952`), these formulas place the expression entries between `out_ptr + 610_318` and `out_ptr + 881_550`. The legacy function metadata that stage2 still updates lives at `out_ptr + 851_968`, so the AST already overlaps that area by about 29 KB. That overlap is harmless only because the last few bytes that spill over fall into unused padding at the end of function records.

Adding blank lines increases `input_len`, which moves the entire AST block forward by **10 bytes per blank line** (the parser inserts "    \n"). Two extra blank lines therefore shift the AST table by 20 bytes—still inside the padding that stage2 never reads. The third blank line, however, consumes another 10 bytes of the remaining slack, so the final logical-OR node in `is_whitespace` lands squarely on the `code_len` field for function index 11 (address `out_ptr + 852_336`). That field becomes `0x08_d0_00_00` (`148_636_160` decimal) instead of the real 14 KB body size, and `emit_code_section` aborts when it later tries to iterate past the corrupted entry.【F:compiler/ast_compiler.bp†L6318-L6394】【ae79d4†L16-L33】

The instruction cursor at `out_ptr + 4_096` is corrupted for the same reason—the overflowing expression node writes the ASCII substring "es_b" of `ast_expr_entries_base` into that slot, which the host then reports as the bogus instruction offset `0x625f7365`. Together these corruptions explain why stage2 can survive one or two extra blank lines but not the third: there simply is no slack left between the AST expression table and the fixed-position scratch structures once the source grows by 15 bytes.

## Implemented fix
The compiler now takes option (1) above. `ast_output_reserve` no longer uses a
fixed `input_len + 65_536` gap; it clamps the AST arena to start **after** the
legacy stage1 table at `out_ptr + 851_968`, leaving the full 16 KiB region for
stage1 metadata before any AST writes occur.【F:compiler/ast_compiler.bp†L1965-L2003】【F:compiler/ast_compiler.bp†L2057-L2064】

For the current bootstrap source this pushes the AST program base to
`out_ptr + 868_352`. Expression entries now occupy
`out_ptr + 1_204_240 .. out_ptr + 1_728_528`, clearing the scratch metadata by
over 335 KiB and eliminating the “third blank line” corruption window. The
expression/type/temporary slabs also shrink to 32K entries, keeping the total
AST footprint under 2 MiB. Larger input files still increase the AST base
(`input_len + 65_536`), so arbitrarily long sources no longer trample the
scratch layout.
