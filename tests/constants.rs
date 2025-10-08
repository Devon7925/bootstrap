#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::run_wasm_main;

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};

#[test]
fn constant_main_returns_literal_value() {
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
fn global_constants_can_be_referenced_from_main() {
    let source = r#"
const ANSWER: i32 = 42;

fn main() -> i32 {
    ANSWER
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn constants_can_reference_other_constants() {
    let source = r#"
const BASE: i32 = 40;
const VALUE: i32 = BASE;

fn main() -> i32 {
    VALUE + 2
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn duplicate_constants_are_rejected() {
    let source = r#"
const VALUE: i32 = 1;
const VALUE: i32 = 2;

fn main() -> i32 {
    VALUE
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("duplicate constants should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn non_literal_constant_initializers_are_rejected() {
    let source = r#"
const VALUE: i32 = 1 + 2;

fn main() -> i32 {
    VALUE
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("non-literal constant initializers should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn function_names_cannot_conflict_with_constants() {
    let source = r#"
const helper: i32 = 1;

fn helper() -> i32 {
    0
}

fn main() -> i32 {
    helper
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("function names should not conflict with constants");
    assert!(error.produced_len <= 0);
}
