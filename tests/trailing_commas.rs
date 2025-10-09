#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::compile_with_ast_compiler;
use wasm_harness::run_wasm_main_with_gc;

#[test]
fn trailing_commas_in_params_and_calls_are_accepted() {
    let source = r#"
fn add(
    a: i32,
    b: i32,
) -> i32 {
    a + b
}

fn main() -> i32 {
    add(1, 2,)
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let result = run_wasm_main_with_gc(&wasm);

    assert_eq!(result, 3);
}
