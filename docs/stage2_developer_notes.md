# Stage-2 Developer Notes

## Type metadata layout

Stage 2 builds a scratch type table in linear memory immediately before the legacy stage-1 function metadata. Each entry is 16 bytes wide and follows the offsets defined in `compiler/ast_compiler.bp` (`TYPE_ENTRY_TYPE_ID_OFFSET`, `TYPE_ENTRY_NAME_PTR_OFFSET`, `TYPE_ENTRY_NAME_LEN_OFFSET`, and `TYPE_ENTRY_EXTRA_OFFSET`). The table base is computed from `scratch_types_base(out_ptr)` and its capacity is fixed at 2,048 entries so the runtime can binary-search or linearly scan without bumping into the function table.

```
+0  (i32) type_id      -> enum describing the high-level type (builtin, struct, array, etc.)
+4  (i32) name_ptr     -> absolute pointer to the UTF-8 type name in the scratch names arena
+8  (i32) name_len     -> byte length of the name string
+12 (i32) extra_ptr    -> type-specific payload (0 when unused)
```

Array entries populate `extra_ptr` with the address of a two-word payload written elsewhere in scratch memory. The payload stores the resolved element type ID followed by the compile-time length. Consumers read that block to materialize `[T; N]` facts (element type, stride, and static bound) without re-parsing the AST. Other aggregate kinds (struct, tuple, enum) can later reuse the same convention by pointing `extra_ptr` at a longer record that begins with a word count.

This layout keeps array metadata compact while leaving `TYPE_ENTRY_EXTRA_OFFSET` free to signal richer schemas as the compiler evolves. As long as we zero the `extra_ptr` for scalar and builtin types, existing tooling (e.g., `tests/wasm_harness.rs`) keeps working because it only inspects the name span today but can opt into array support by following the pointer when it is non-zero.
