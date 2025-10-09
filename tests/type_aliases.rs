#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::run_wasm_main_with_gc;

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};

#[test]
fn type_aliases_can_rename_builtin_types() {
    let source = r#"
        type MyInt = i32;

        fn main() -> i32 {
            let value: MyInt = 41;
            value + 1
        }
    "#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn type_aliases_can_rename_array_types() {
    let source = r#"
        type MyArray = [i32; 2];

        fn main() -> i32 {
            let value: MyArray = [41, 42];
            value[1] + 1
        }
    "#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 43);
}

#[test]
fn type_aliases_can_be_used_as_array_element_types() {
    let source = r#"
        type MyInt = i32;

        fn main() -> i32 {
            let value: [MyInt; 2] = [41, 42];
            value[1] + 1
        }
    "#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 43);
}

#[test]
fn type_aliases_can_chain() {
    let source = r#"
        type Base = i32;
        type Wrapper = Base;

        fn add_one(value: Wrapper) -> Wrapper {
            value + 1
        }

        fn main() -> i32 {
            add_one(41)
        }
    "#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn type_aliases_can_reference_aliases_after() {
    let source = r#"
        type Wrapper = Base;
        type Base = i32;

        fn add_one(value: Wrapper) -> Wrapper {
            value + 1
        }

        fn main() -> i32 {
            add_one(41)
        }
    "#;

    let wasm = compile_with_ast_compiler(source);
    let result = run_wasm_main_with_gc(&wasm);
    assert_eq!(result, 42);
}

#[test]
fn missing_type_aliases_are_rejected() {
    let source = r#"
        fn main() -> Missing {
            0
        }
    "#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("use of missing type aliases should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn self_type_aliases_are_rejected() {
    let source = r#"
        type Loop = Loop;

        fn main() -> i32 {
            0
        }
    "#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("cyclic type aliases should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn cyclic_type_aliases_are_rejected() {
    let source = r#"
        type LoopA = LoopB;
        type LoopB = LoopC;
        type LoopC = LoopA;

        fn main() -> i32 {
            0
        }
    "#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("cyclic type aliases should be rejected");
    assert!(error.produced_len <= 0);
}
