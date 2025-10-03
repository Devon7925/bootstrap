use bootstrap::compile;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn program_requires_main() {
    let source = r#"
fn helper() -> i32 {
    1
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected missing main error"),
        Err(err) => err,
    };
    assert!(
        error.message.contains("stage2 compilation failed"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn main_cannot_accept_parameters() {
    let source = r#"
fn main(value: i32) -> i32 {
    value
}
"#;

    let compilation = compile(source).expect("failed to compile program with parameters");
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

    let main: TypedFunc<i32, i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, 42).expect("failed to call main");
    assert_eq!(result, 42);
}

#[test]
fn main_must_return_i32() {
    let source = r#"
fn main() -> bool {
    true
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected main return type error"),
        Err(err) => err,
    };
    assert!(
        error.message.contains("stage2 compilation failed"),
        "unexpected error message: {}",
        error.message
    );
}
