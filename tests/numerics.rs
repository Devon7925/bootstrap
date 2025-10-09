#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};
use wasm_harness::{instantiate_module, run_wasm_main_with_gc, wasmtime_engine_with_gc};
use wasmtime::TypedFunc;

#[test]
fn numeric_operations_execute() {
    let source = r#"
fn add_offset(a: i32, b: i32) -> i32 {
    a + b + 1
}

fn sum_values() -> i32 {
    let mut total: i32 = 1;
    total = total + 2;
    total
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = wasmtime_engine_with_gc();
    let (mut store, instance) = instantiate_module(&engine, &wasm);

    let add_offset_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_offset")
        .expect("expected exported add_offset");
    let main_func: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");
    let sum_values_func: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "sum_values")
        .expect("expected exported sum_values");

    let add_offset_result = add_offset_func
        .call(&mut store, (10, 5))
        .expect("failed to execute add_offset");
    assert_eq!(add_offset_result, 16);

    let main_result = main_func
        .call(&mut store, ())
        .expect("failed to execute main");
    assert_eq!(main_result, 0);

    let sum_result = sum_values_func
        .call(&mut store, ())
        .expect("failed to execute sum_values");
    assert_eq!(sum_result, 3);
}

#[test]
fn float_remainder_is_rejected() {
    let source = r#"
fn float_mod() -> f32 {
    5.0f32 % 2.0f32
}

fn main() -> i32 {
    0
}
"#;

    let error =
        try_compile_with_ast_compiler(source).expect_err("expected float remainder to be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn literal_addition_executes() {
    let source = r#"
fn main() -> i32 {
    1 + 2 + 3
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 6);
}

#[test]
fn addition_with_function_call_executes() {
    let source = r#"
fn helper() -> i32 {
    5
}

fn main() -> i32 {
    helper() + 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 12);
}

#[test]
fn comparison_operators_evaluate() {
    let source = r#"
fn evaluate(a: i32, b: i32) -> i32 {
    let mut total: i32 = 0;
    if a == b {
        total = total + 1;
        0
    } else {
        total = total + 2;
        0
    };
    if a != b {
        total = total + 4;
        0
    } else {
        total = total + 8;
        0
    };
    if a < b {
        total = total + 16;
        0
    } else {
        total = total + 32;
        0
    };
    if a > b {
        total = total + 64;
        0
    } else {
        total = total + 128;
        0
    };
    if a <= b {
        total = total + 256;
        0
    } else {
        total = total + 512;
        0
    };
    if a >= b {
        total = total + 1024;
        0
    } else {
        total = total + 2048;
        0
    };
    total
}

fn precedence() -> i32 {
    let mut total: i32 = 0;
    if 1 + 2 == 3 {
        total = total + 1000;
        0
    } else {
        total = total + 1;
        0
    };
    if 20 - 5 >= 15 {
        total = total + 2000;
        0
    } else {
        total = total + 2;
        0
    };
    if 3 * 3 < 10 {
        total = total + 4000;
        0
    } else {
        total = total + 4;
        0
    };
    total
}

fn main() -> i32 {
    evaluate(4, 4) + evaluate(2, 5) + precedence()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 10903);
}

#[test]
fn missing_function_in_addition_is_rejected() {
    let source = r#"
fn main() -> i32 {
    missing() + 1
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("unknown calls in addition expressions should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn literal_subtraction_executes() {
    let source = r#"
fn main() -> i32 {
    50 - 8
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn subtraction_with_function_call_executes() {
    let source = r#"
fn helper() -> i32 {
    20
}

fn main() -> i32 {
    helper() - 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 13);
}

#[test]
fn subtraction_rejects_unknown_function_calls() {
    let source = r#"
fn main() -> i32 {
    5 - missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("unknown calls in subtraction expressions should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn literal_multiplication_executes() {
    let source = r#"
fn main() -> i32 {
    6 * 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn multiplication_with_function_call_executes() {
    let source = r#"
fn helper() -> i32 {
    6
}

fn main() -> i32 {
    helper() * 7
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn literal_division_executes() {
    let source = r#"
fn main() -> i32 {
    126 / 3
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn multiplication_precedence_is_respected() {
    let source = r#"
fn main() -> i32 {
    2 + 3 * 4
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 14);
}

#[test]
fn multiplication_rejects_unknown_function_calls() {
    let source = r#"
fn main() -> i32 {
    3 * missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("unknown calls in multiplication expressions should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn mixed_addition_and_subtraction_executes() {
    let source = r#"
fn main() -> i32 {
    10 + 5 - 3 + 2 - 4
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 10);
}
