#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::run_wasm_main;

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};

#[test]
fn ast_compiler_emits_constant_main() {
    let source = r#"
fn main() -> i32 {
    42
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_requires_main_function() {
    let source = r#"
fn helper() -> i32 {
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject programs without main");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_rejects_duplicate_function_names() {
    let source = r#"
fn helper() -> i32 {
    1
}

fn helper() -> i32 {
    2
}

fn main() -> i32 {
    3
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject duplicate function names");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_rejects_multiple_main_functions() {
    let source = r#"
fn main() -> i32 {
    1
}

fn helper() -> i32 {
    2
}

fn main() -> i32 {
    3
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject programs with multiple mains");
    assert!(error.produced_len <= 0);
}
