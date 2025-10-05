#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::compile_with_ast_compiler;
use wasmi::{Engine, Linker, Module, Store};

#[test]
fn partial_bootstrap() {
    let source = include_str!("../compiler/partial_ast_compiler.bp");

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");
}
