#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, run_wasm_main_with_gc};

fn contains_sequence(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn string_literal_emits_array_new_fixed() {
    let source = r#"
fn build() -> [u8; 5] {
    "hello"
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let pattern = [
        0x41, 0xe8, 0x00, 0x41, 0xe5, 0x00, 0x41, 0xec, 0x00, 0x41, 0xec, 0x00, 0x41, 0xef,
        0x00, 0xfb, 0x08, 0x00, 0x05,
    ];
    assert!(
        contains_sequence(&wasm, &pattern),
        "expected wasm to contain array.new_fixed for \"hello\" literal",
    );
}

#[test]
fn string_literal_supports_escapes() {
    let source = r#"
fn build() -> [u8; 4] {
    "\n\t\\\""
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let pattern = [0x41, 0x0a, 0x41, 0x09, 0x41, 0xdc, 0x00, 0x41, 0x22, 0xfb, 0x08, 0x00, 0x04];
    assert!(
        contains_sequence(&wasm, &pattern),
        "expected wasm to contain escaped bytes in order",
    );
}

#[test]
fn empty_string_literal_has_zero_length() {
    let source = r#"
fn main() -> i32 {
    len("")
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 0, "expected empty string literal to have length 0");
}

#[test]
fn assign_to_array_local() {
    let source = r#"
fn main() -> i32 {
    let test: [u8; 4] = "test";
    len(test)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 4, "expected 'test' string literal to have length 4");
}

#[test]
fn string_indices_match_char_casts() {
    let source = r#"
fn main() -> i32 {
    let word: [u8; 5] = "hello";
    let mut score: i32 = 0;

    if word[0] == ('h' as u8) {
        score = score + 1;
        0
    } else {
        0
    };

    if word[1] == ('e' as u8) {
        score = score + 1;
        0
    } else {
        0
    };

    if word[2] == ('l' as u8) {
        score = score + 1;
        0
    } else {
        0
    };

    if word[3] == ('l' as u8) {
        score = score + 1;
        0
    } else {
        0
    };

    if word[4] == ('o' as u8) {
        score = score + 1;
        0
    } else {
        0
    };

    score
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 5, "expected every index to match its character literal");
}
