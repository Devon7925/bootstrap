#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{
    compile_with_ast_compiler,
    run_wasm_main_with_gc,
    try_compile_with_ast_compiler,
};

fn contains_sequence(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn array_literal_emits_array_new() {
    let source = r#"
fn build() -> [i32; 4] {
    [2; 4]
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let pattern = [0x41, 0x02, 0x41, 0x04, 0xfb, 0x06, 0x00];
    assert!(
        contains_sequence(&wasm, &pattern),
        "expected wasm to contain array.new for [2; 4] literal",
    );
}

#[test]
fn array_literal_uses_expression_default_value() {
    let source = r#"
fn build(value: i32) -> [i32; 3] {
    [value; 3]
}

fn main() -> i32 {
    build(5);
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let mut found = false;
    for idx in 0u8..=10 {
        let pattern = [0x20, idx, 0x41, 0x03, 0xfb, 0x06, 0x00];
        if contains_sequence(&wasm, &pattern) {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected wasm to load default expression before array.new",
    );
}

#[test]
fn array_literal_can_be_passed_to_function_arguments() {
    let source = r#"
fn take(arg: [i32; 4]) -> i32 {
    0
}

fn main() -> i32 {
    take([7; 4])
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let mut found = false;
    for call_index in 0u8..=10 {
        let pattern = [0x41, 0x07, 0x41, 0x04, 0xfb, 0x06, 0x00, 0x10, call_index];
        if contains_sequence(&wasm, &pattern) {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "expected wasm to allocate the array literal before calling take",
    );

    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 0, "expected main to execute successfully");
}

#[test]
fn array_literal_rejects_negative_length() {
    let source = r#"
fn build() -> [i32; 4] {
    [2; -1]
}

fn main() -> i32 {
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("array literals should reject negative lengths");
    assert!(error.produced_len <= 0);
}

#[test]
fn array_literal_length_must_match_declared_type() {
    let source = r#"
fn build() -> [i32; 4] {
    [2; 3]
}

fn main() -> i32 {
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("array literals should not allow mismatched lengths");
    assert!(error.produced_len <= 0);
}
