#[path = "stage1_helpers.rs"]
mod stage1_helpers;

use stage1_helpers::{compile_with_stage1, try_compile_with_stage1};
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

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

    let wasm = compile_with_stage1(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

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

    let error = try_compile_with_stage1(source)
        .expect_err("expected float remainder to be rejected");
    assert!(error.produced_len <= 0);
}
