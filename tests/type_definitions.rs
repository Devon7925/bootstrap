#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::{ast_compiler_wasm, try_compile_with_ast_compiler};
use wasm_harness::{CompilerInstance, DEFAULT_OUTPUT_STRIDE};

fn compile_with_instance(source: &str) -> (CompilerInstance, i32) {
    let mut compiler = CompilerInstance::new(ast_compiler_wasm());
    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;
    compiler
        .compile_with_layout(&mut input_cursor, &mut output_cursor, source)
        .expect("ast_compiler should compile source with type definitions");
    let output_ptr = output_cursor - DEFAULT_OUTPUT_STRIDE;
    (compiler, output_ptr)
}

#[test]
#[ignore]
fn type_definitions_are_registered() {
    let source = r#"
        type Count = i32;
        type Flag = bool ;
        fn helper() -> i32 {
            1
        }

        fn main() -> i32 {
            helper()
        }
    "#;

    let (compiler, output_ptr) = compile_with_instance(source);

    let type_count = compiler.read_types_count(output_ptr);
    assert_eq!(type_count, 2, "expected two registered type definitions");

    let first = compiler.read_type_entry(output_ptr, 0);
    let second = compiler.read_type_entry(output_ptr, 1);

    let count_name_start = first.name_start as usize;
    let count_name_end = count_name_start + first.name_len as usize;
    assert_eq!(&source[count_name_start..count_name_end], "Count");

    let count_value_start = first.value_start as usize;
    let count_value_end = count_value_start + first.value_len as usize;
    assert_eq!(&source[count_value_start..count_value_end], "i32");

    let flag_name_start = second.name_start as usize;
    let flag_name_end = flag_name_start + second.name_len as usize;
    assert_eq!(&source[flag_name_start..flag_name_end], "Flag");

    let flag_value_start = second.value_start as usize;
    let flag_value_end = flag_value_start + second.value_len as usize;
    assert_eq!(&source[flag_value_start..flag_value_end], "bool");
}

#[test]
#[ignore]
fn duplicate_type_definitions_are_rejected() {
    let source = r#"
        type Count = i32;
        type Count = bool;
        fn main() -> i32 {
            0
        }
    "#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("duplicate type definitions should be rejected");
    assert!(error.produced_len <= 0, "compiler should signal failure");
}
