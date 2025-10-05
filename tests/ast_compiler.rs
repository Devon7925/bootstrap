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

#[test]
fn ast_compiler_compiles_function_calls() {
    let source = r#"
fn helper() -> i32 {
    40
}

fn main() -> i32 {
    helper()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 40);
}

#[test]
fn ast_compiler_supports_forward_function_calls() {
    let source = r#"
fn main() -> i32 {
    helper()
}

fn helper() -> i32 {
    42
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_rejects_unknown_function_call() {
    let source = r#"
fn main() -> i32 {
    missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject calls to missing functions");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_compiles_literal_addition() {
    let source = r#"
fn main() -> i32 {
    1 + 2 + 3
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 6);
}

#[test]
fn ast_compiler_compiles_addition_with_function_call() {
    let source = r#"
fn helper() -> i32 {
    5
}

fn main() -> i32 {
    helper() + 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 12);
}

#[test]
fn ast_compiler_rejects_unknown_function_in_addition() {
    let source = r#"
fn main() -> i32 {
    missing() + 1
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject unknown calls in addition expressions");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_compiles_parenthesized_literal() {
    let source = r#"
fn main() -> i32 {
    (42)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_compiles_nested_parentheses_in_addition() {
    let source = r#"
fn helper() -> i32 {
    10
}

fn main() -> i32 {
    (helper()) + (1 + (2 + 3))
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 16);
}

#[test]
fn ast_compiler_compiles_literal_subtraction() {
    let source = r#"
fn main() -> i32 {
    50 - 8
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_compiles_subtraction_with_function_call() {
    let source = r#"
fn helper() -> i32 {
    20
}

fn main() -> i32 {
    helper() - 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 13);
}

#[test]
fn ast_compiler_rejects_unknown_function_in_subtraction() {
    let source = r#"
fn main() -> i32 {
    5 - missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject unknown calls in subtraction expressions");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_compiles_literal_multiplication() {
    let source = r#"
fn main() -> i32 {
    6 * 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_compiles_multiplication_with_function_call() {
    let source = r#"
fn helper() -> i32 {
    6
}

fn main() -> i32 {
    helper() * 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_compiles_literal_division() {
    let source = r#"
fn main() -> i32 {
    126 / 3
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_respects_multiplication_precedence() {
    let source = r#"
fn main() -> i32 {
    2 + 3 * 4
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 14);
}

#[test]
fn ast_compiler_rejects_unknown_function_in_multiplication() {
    let source = r#"
fn main() -> i32 {
    3 * missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject unknown calls in multiplication expressions");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_honors_parentheses_with_multiplication() {
    let source = r#"
fn main() -> i32 {
    (2 + 3) * 4
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 20);
}

#[test]
fn ast_compiler_compiles_mixed_addition_and_subtraction() {
    let source = r#"
fn main() -> i32 {
    10 + 5 - 3 + 2 - 4
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10);
}
