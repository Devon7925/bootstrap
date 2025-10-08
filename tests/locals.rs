#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::run_wasm_main;

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};

#[test]
fn locals_are_scoped_to_blocks() {
    let source = r#"
fn main() -> i32 {
    let outer: i32 = 5;
    {
        let inner: i32 = outer + 10;
        inner
    } + outer
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 20);
}

#[test]
fn locals_can_be_shadowed_in_nested_blocks() {
    let source = r#"
fn main() -> i32 {
    let value: i32 = 5;
    {
        let value: i32 = value + 1;
        value
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 6);
}

#[test]
fn using_out_of_scope_locals_is_rejected() {
    let source = r#"
fn main() -> i32 {
    {
        let inner: i32 = 5;
        inner
    };
    inner
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("references to out-of-scope locals should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn assignment_to_immutable_locals_is_rejected() {
    let source = r#"
fn main() -> i32 {
    let value: i32 = 1;
    value = 2;
    value
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("assignment to immutable locals should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn blocks_must_end_with_an_expression() {
    let source = r#"
fn main() -> i32 {
    let value: i32 = 1;
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("blocks must have a final expression");
    assert!(error.produced_len <= 0);
}
