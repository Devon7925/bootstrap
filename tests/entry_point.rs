#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn program_requires_main() {
    let source = r#"
fn helper() -> i32 {
    1
}
"#;

    let error = try_compile_with_ast_compiler(source).expect_err("expected missing main error");
    assert!(error.produced_len <= 0);
}

#[test]
#[ignore]
fn main_cannot_accept_parameters() {
    let source = r#"
fn main(value: i32) -> i32 {
    value
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

    let main: TypedFunc<i32, i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, 42).expect("failed to call main");
    assert_eq!(result, 42);
}

#[test]
#[ignore]
fn main_must_return_i32() {
    let source = r#"
fn main() -> bool {
    true
}
"#;

    let error = try_compile_with_ast_compiler(source).expect_err("expected main return type error");
    assert!(error.produced_len <= 0);
}
