# Constant import regression investigation

Running `bun test` after introducing cross-module constant matching causes every suite that relies on the stage2 compiler to fail with `stage2 compilation failed (status -1, functions=0, instr_offset=0, compiled_functions=0)` while compiling the AST compiler itself.【b548de†L1-L108】

The regression stems from the updated `find_constant_entry_index` helper. The new implementation compares candidate names against every constant in the arena using the stored source buffer, so references imported through `use` now resolve correctly across module boundaries.【F:compiler/ast_compiler.bp†L424-L477】 However, the constant parser still calls this helper to reject duplicate declarations. Because the check now sees constants from previously imported modules, the parser flags legitimate declarations that merely share a name with an imported constant.【F:compiler/ast_compiler.bp†L7840-L7889】

The AST compiler imports `/compiler/wasm_output.bp`, which defines several shared constants such as `WORD_SIZE`, `COMPILER_MEMORY_PAGES`, and `LOCAL_COUNTS_BASE` at the top of the file.【F:compiler/wasm_output.bp†L1-L31】 Those identifiers are re-declared inside `/compiler/ast_compiler.bp` to describe its own memory layout.【F:compiler/ast_compiler.bp†L2058-L2096】 After the cross-module comparison change, the duplicate check rejects these local definitions, so parsing aborts and the compiler fails to bootstrap.

Until the duplicate detection logic is updated to distinguish between imported and locally declared constants, the stage2 compiler will continue to reject the AST compiler's source code.
