#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::compile_with_ast_compiler;
use wasm_harness::run_wasm_main;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn parenthesized_expressions_evaluate_correctly() {
    let source = r#"
fn compute() -> i32 {
    let base: i32 = 2;
    (base + 3) * (4 + 1)
}

fn bool_gate(flag: bool) -> i32 {
    if (flag && (false || true)) {
        1
    } else {
        0
    }
}

fn main() -> i32 {
    (compute() / (3 - 1)) + bool_gate(true)
}
"#;

    let wasm = compile_with_ast_compiler(source);

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

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");

    assert_eq!(result, 13);
}

#[test]
fn parenthesized_literal_executes() {
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
fn nested_parentheses_in_addition_execute() {
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
fn parentheses_affect_multiplication_order() {
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
