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
fn ast_compiler_supports_comparison_operators() {
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
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10903);
}

#[test]
fn ast_compiler_supports_logical_operators() {
    let source = r#"
fn main() -> i32 {
    let mut count: i32 = 0;
    let result1: bool = true || { count = count + 1; false };
    let result2: bool = false || { count = count + 1; true };
    let result3: bool = false && { count = count + 1; true };
    let result4: bool = true && { count = count + 1; true };
    let toggled: bool = !false;
    let double_negated: bool = !(!true);
    let inverted: bool = !result4;
    if result1 && result2 && !result3 && result4 && toggled && !inverted && double_negated {
        count
    } else {
        0
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 2);
}

#[test]
fn ast_compiler_supports_bitwise_operations_and_shifts() {
    let source = r#"
fn evaluate(a: i32, b: i32, shift: i32) -> i32 {
    let mask: i32 = (a & b) | ((a | b) >> shift);
    (mask << 1) + (a >> shift)
}

fn main() -> i32 {
    let first: i32 = evaluate(29, 23, 2);
    let second: i32 = evaluate(-64, 7, 3);
    first + second
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);

    let expected = {
        let eval = |a: i32, b: i32, shift: i32| {
            let mask = (a & b) | ((a | b) >> shift);
            (mask << 1) + (a >> shift)
        };
        eval(29, 23, 2) + eval(-64, 7, 3)
    };

    assert_eq!(result, expected);
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
fn ast_compiler_handles_predicate_calls_in_loops() {
    let source = r#"
fn predicate(value: i32) -> bool {
    if value >= 3 {
        true
    } else {
        false
    }
}

fn main() -> i32 {
    let mut value: i32 = 0;
    loop {
        if predicate(value) {
            break;
        };
        value = value + 1;
    }
    value
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 3);
}

#[test]
fn ast_compiler_supports_functions_without_return_type() {
    let source = r#"
fn helper() {
    let mut counter: i32 = 0;
    counter = counter + 1;
}

fn main() -> i32 {
    helper();
    42
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn ast_compiler_handles_many_function_parameters() {
    let source = r#"
fn wide(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    a6: i32,
    a7: i32,
    a8: i32,
    a9: i32,
    a10: i32,
    a11: i32,
    a12: i32,
    a13: i32,
    a14: i32,
    a15: i32,
    a16: i32,
    a17: i32,
    a18: i32,
    a19: i32,
) -> i32 {
    a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9
        + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19
}

fn main() -> i32 {
    wide(
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
    )
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 190);
}

#[test]
fn ast_compiler_supports_if_statements_in_blocks() {
    let source = r#"
fn adjust(input: i32) -> i32 {
    let mut value: i32 = input;
    if value > 0 {
        value = value - 1;
    };
    if value < 0 {
        value = 0;
    };
    value
}

fn main() -> i32 {
    adjust(2) + adjust(-1)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 1);
}

#[test]
fn ast_compiler_supports_continue_statements() {
    let source = r#"
fn sum_even(limit: i32) -> i32 {
    let mut acc: i32 = 0;
    let mut i: i32 = 0;
    loop {
        if i >= limit {
            break;
            0
        } else {
            0
        };
        i = i + 1;
        let remainder: i32 = i - (i / 2) * 2;
        if remainder == 1 {
            continue;
            0
        } else {
            0
        };
        acc = acc + i;
    }
    acc
}

fn loop_skip() -> i32 {
    let mut total: i32 = 0;
    let mut i: i32 = 0;
    loop {
        i = i + 1;
        if i > 5 {
            break;
            0
        } else {
            0
        };
        if i == 3 {
            continue;
            0
        } else {
            0
        };
        total = total + i;
    }
    total
}

fn main() -> i32 {
    sum_even(6) + loop_skip()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 24);
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
fn ast_compiler_rejects_continue_outside_loop() {
    let source = r#"
fn main() -> i32 {
    continue;
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("ast compiler should reject continue outside loops");
    assert!(error.produced_len <= 0);
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
fn ast_compiler_supports_return_statements() {
    let source = r#"
fn choose(flag: i32) -> i32 {
    if flag {
        return 10;
    } else {
        return 20;
    }
}

fn accumulate(limit: i32) -> i32 {
    let mut total: i32 = 0;
    let mut current: i32 = limit;
    loop {
        if current <= 0 {
            return total;
        } else {
            total = total + current;
            current = current - 1;
            0
        };
    }
}

fn main() -> i32 {
    choose(1) + choose(0) + accumulate(3)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 36);
}

#[test]
fn ast_compiler_allows_diverging_if_tail_statements() {
    let source = r#"
fn branch(flag: bool) -> i32 {
    if flag {
        return 10;
    } else {
        return 20;
    };
}

fn main() -> i32 {
    branch(true)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10);
}

#[test]
fn ast_compiler_rejects_break_outside_loop() {
    let source = r#"
fn main() -> i32 {
    break;
    0
}
"#;

    let error =
        try_compile_with_ast_compiler(source).expect_err("break outside loop should be rejected");
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
