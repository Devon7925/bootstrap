#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompilerInstance, DEFAULT_OUTPUT_STRIDE, run_wasm_main_with_gc};

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{
    ast_compiler_source, ast_compiler_wasm, compile_with_ast_compiler,
    try_compile_with_ast_compiler,
};

#[test]
fn functions_can_call_other_functions() {
    let source = r#"
fn helper() -> i32 {
    40
}

fn main() -> i32 {
    helper()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 40);
}

#[test]
fn functions_can_accept_parameters() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    add(40, 2)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn forward_function_calls_are_supported() {
    let source = r#"
fn main() -> i32 {
    helper()
}

fn helper() -> i32 {
    42
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn unknown_function_calls_are_rejected() {
    let source = r#"
fn main() -> i32 {
    missing()
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("calls to missing functions should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn call_argument_counts_must_match_function_signature() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    add(1)
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("calls with incorrect arity should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn duplicate_function_names_are_rejected() {
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
        .expect_err("duplicate function names should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn functions_may_omit_return_types() {
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
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn functions_support_many_parameters() {
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
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 190);
}

#[test]
fn functions_can_return_from_multiple_paths() {
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
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 36);
}

#[test]
fn functions_can_use_local_variables() {
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
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn function_section_handles_multibyte_type_indices() {
    use std::fmt::Write as _;

    let helper_count: i32 = (1 << 7) + 2;
    let mut source = String::new();

    let mut idx = 0;
    loop {
        if idx >= helper_count {
            break;
        }
        writeln!(&mut source, "fn helper_{idx}() -> i32 {{").unwrap();
        writeln!(&mut source, "    {idx}").unwrap();
        writeln!(&mut source, "}}").unwrap();
        writeln!(&mut source).unwrap();
        idx += 1;
    }

    writeln!(
        &mut source,
        "fn main() -> i32 {{\n    helper_{}()\n}}",
        helper_count - 1
    )
    .unwrap();

    let wasm = compile_with_ast_compiler(&source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, helper_count - 1);
}

#[test]
fn ast_compiler_source_can_be_compiled_once() {
    let mut compiler = CompilerInstance::new(ast_compiler_wasm());
    let source = ast_compiler_source();

    let wasm = compiler
        .compile_at(0, source.len() as i32, source)
        .expect("ast compiler should compile its own source");

    assert!(
        wasm.len() > DEFAULT_OUTPUT_STRIDE as usize,
        "self-compiled output should be larger than the default stride",
    );
}
