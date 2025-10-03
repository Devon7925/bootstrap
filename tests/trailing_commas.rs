use std::fs;

use bootstrap::compile;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompilerInstance, run_wasm_main};

#[test]
fn trailing_commas_in_params_and_calls_are_accepted() {
    let source = r#"
fn add(
    a: i32,
    b: i32,
) -> i32 {
    a + b
}

fn main() -> i32 {
    add(1, 2,)
}
"#;

    let compilation = compile(source).expect("failed to compile source");
    let wasm = compilation.to_wasm().expect("failed to encode wasm");

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

    assert_eq!(result, 3);
}

#[test]
fn stage1_compiler_accepts_trailing_commas() {
    let stage1_source =
        fs::read_to_string("examples/stage1_minimal.bp").expect("failed to load stage1 source");

    let stage1_compilation = compile(&stage1_source).expect("failed to compile stage1 source");
    let stage1_wasm = stage1_compilation
        .to_wasm()
        .expect("failed to encode stage1 wasm");

    let mut compiler = CompilerInstance::new(stage1_wasm.as_slice());

    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;

    let program = r#"
fn add(
    a: i32,
    b: i32,
) -> i32 {
    a + b
}

fn main() -> i32 {
    add(1, 2,)
}
"#;

    let output = compiler
        .compile_with_layout(&mut input_cursor, &mut output_cursor, program)
        .expect("stage1 should compile trailing comma program");

    let result = run_wasm_main(compiler.engine(), &output);

    assert_eq!(result, 3);
}
