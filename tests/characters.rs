#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};
use wasm_harness::{instantiate_module, wasmtime_engine_with_gc};
use wasmtime::TypedFunc;

#[test]
fn character_literals_execute() {
    let source = r#"
fn char_math() -> i32 {
    let letter: i32 = 'a';
    let newline: i32 = '\n';
    let quote: i32 = '\'';
    letter + newline + quote
}

fn slash() -> i32 {
    '\\'
}

fn main() -> i32 {
    if '\\' == 92 {
        char_math() - '\\' + 'A'
    } else {
        0
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = wasmtime_engine_with_gc();
    let (mut store, instance) = instantiate_module(&engine, &wasm);

    let char_math: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "char_math")
        .expect("expected exported char_math");
    let slash: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "slash")
        .expect("expected exported slash");
    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let char_math_result = char_math
        .call(&mut store, ())
        .expect("failed to execute char_math");
    assert_eq!(char_math_result, 146);

    let slash_result = slash.call(&mut store, ()).expect("failed to execute slash");
    assert_eq!(slash_result, 92);

    let main_result = main.call(&mut store, ()).expect("failed to execute main");
    assert_eq!(main_result, 119);
}

#[test]
fn invalid_character_literals_are_rejected() {
    let source = r#"
fn main() -> i32 {
    let bad: i32 = 'ab';
    bad
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("expected invalid character literal to be rejected");
    assert!(error.produced_len <= 0);
}
