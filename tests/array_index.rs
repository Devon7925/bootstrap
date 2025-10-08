#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{
    compile_with_ast_compiler,
    run_wasm_main_with_gc,
    try_compile_with_ast_compiler,
};

#[test]
fn array_index_reads_element() {
    let source = r#"
fn select(values: [i32; 3], idx: i32) -> i32 {
    values[idx]
}

fn main() -> i32 {
    select([7; 3], 1)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 7, "expected array indexing to return the selected element");
}

#[test]
fn array_index_requires_integer_indices() {
    let source = r#"
fn main() -> i32 {
    let values: [i32; 2] = [1; 2];
    values[true]
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("array indices must be 32-bit integers");
    assert!(error.produced_len <= 0);
}
