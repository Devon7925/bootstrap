#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::compile_with_ast_compiler;
use wasm_harness::run_wasm_main;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn bitwise_and_shifts_execute() {
    let source = r#"
fn bit_ops(a: i32, b: i32) -> i32 {
    let and_value: i32 = a & b;
    let or_value: i32 = a | b;
    (and_value << 1) + or_value
}

fn shifts(value: i32, amount: i32) -> i32 {
    (value << amount) + (value >> amount)
}

fn main() -> i32 {
    bit_ops(12, 5) + shifts(-8, 1)
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let module = Module::new(&engine, wasm.as_slice()).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let bit_ops: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "bit_ops")
        .expect("expected exported bit_ops");
    let shifts: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "shifts")
        .expect("expected exported shifts");
    let main_fn: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let bit_result = bit_ops
        .call(&mut store, (0b1100, 0b0101))
        .expect("failed to call bit_ops");
    let expected_bit = ((0b1100 & 0b0101) << 1) + (0b1100 | 0b0101);
    assert_eq!(bit_result, expected_bit);

    let shift_result = shifts
        .call(&mut store, (-32, 2))
        .expect("failed to call shifts");
    assert_eq!(shift_result, (-32 << 2) + (-32 >> 2));

    let main_result = main_fn.call(&mut store, ()).expect("failed to call main");
    let expected_main = ((12 & 5) << 1) + (12 | 5) + ((-8 << 1) + (-8 >> 1));
    assert_eq!(main_result, expected_main);
}

#[test]
fn bitwise_expressions_execute() {
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
