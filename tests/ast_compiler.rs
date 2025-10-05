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
fn ast_compiler_rejects_main_with_parameters() {
    let source = r#"
fn main(value: i32) -> i32 {
    value
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject main functions with parameters");
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
fn ast_compiler_compiles_functions_with_parameters() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    add(40, 2)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
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
fn ast_compiler_rejects_call_with_wrong_argument_count() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    add(1)
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject calls with incorrect arity");
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
fn ast_compiler_supports_boolean_types_and_literals() {
    let source = r#"
fn invert(flag: bool) -> bool {
    if flag {
        false
    } else {
        true
    }
}

fn main() -> i32 {
    let truth: bool = true;
    let falsity: bool = invert(truth);
    if falsity {
        0
    } else {
        1
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 1);
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
fn ast_compiler_supports_loop_and_break() {
    let source = r#"
fn sum_up_to(limit: i32) -> i32 {
    let mut total: i32 = 0;
    let mut count: i32 = 0;
    let mut remaining: i32 = limit;
    loop {
        if remaining {
            total = total + count;
            count = count + 1;
            remaining = remaining - 1;
            0
        } else {
            break;
            0
        };
    }
    total
}

fn main() -> i32 {
    sum_up_to(5)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10);
}

#[test]
fn ast_compiler_loop_break_value_returns() {
    let source = r#"
fn choose() -> i32 {
    loop {
        break 42;
    }
}

fn main() -> i32 {
    choose()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_nested_loops_break_with_values() {
    let source = r#"
fn nested(limit: i32) -> i32 {
    let mut outer: i32 = limit;
    let mut total: i32 = 0;
    loop {
        if outer {
            let mut inner: i32 = outer;
            loop {
                if inner {
                    total = total + outer;
                    inner = inner - 1;
                    0
                } else {
                    break;
                    0
                };
            }
            outer = outer - 1;
            0
        } else {
            break total;
            0
        };
    }
}

fn main() -> i32 {
    nested(3)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 14);
}

#[test]
fn ast_compiler_rejects_break_outside_loop() {
    let source = r#"
fn main() -> i32 {
    break;
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("break outside loop should be rejected");
    assert!(error.produced_len <= 0);
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

#[test]
fn ast_compiler_compiles_if_with_literal_condition() {
    let source = r#"
fn main() -> i32 {
    if 1 {
        42
    } else {
        0
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_compiles_if_else_with_parameter_condition() {
    let source = r#"
fn choose(flag: i32) -> i32 {
    if flag {
        10
    } else {
        20
    }
}

fn main() -> i32 {
    choose(0)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 20);
}

#[test]
fn ast_compiler_compiles_nested_if_expressions() {
    let source = r#"
fn pick(a: i32, b: i32) -> i32 {
    if a {
        if b {
            1
        } else {
            2
        }
    } else {
        if b {
            3
        } else {
            4
        }
    }
}

fn main() -> i32 {
    pick(0, 1) + pick(1, 0) * 10
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 23);
}

#[test]
fn ast_compiler_compiles_functions_with_local_variables() {
    let source = r#"
fn compute() -> i32 {
    let base: i32 = 40;
    let mut total: i32 = base + 1;
    total = total + 1;
    total
}

fn main() -> i32 {
    compute()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_scopes_locals_to_blocks() {
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
fn ast_compiler_allows_shadowing_locals_in_nested_blocks() {
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
fn ast_compiler_rejects_use_of_out_of_scope_local() {
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
        .expect_err("ast compiler should reject references to out-of-scope locals");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_rejects_assignment_to_immutable_local() {
    let source = r#"
fn main() -> i32 {
    let value: i32 = 1;
    value = 2;
    value
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject assignment to immutable locals");
    assert!(error.produced_len <= 0);
}

#[test]
fn ast_compiler_requires_block_to_have_final_expression() {
    let source = r#"
fn main() -> i32 {
    let value: i32 = 1;
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject blocks without a final expression");
    assert!(error.produced_len <= 0);
}
