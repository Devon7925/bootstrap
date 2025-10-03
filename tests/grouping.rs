#[path = "stage1_helpers.rs"]
mod stage1_helpers;

use stage1_helpers::compile_with_stage1;
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

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");

    assert_eq!(result, 13);
}
